//! Live NMBS/SNCB data via the open iRail API (<https://docs.irail.be>).
//!
//! All HTTP traffic runs on one background thread that handles requests in
//! order and spaces them out, because iRail asks clients to stay under about
//! three requests per second.

use crate::model::{ArrDep, Icon, Status, Train};
use chrono::DateTime;
use chrono_tz::Europe::Brussels;
use serde_json::Value;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

const API: &str = "https://api.irail.be/v1";

/// API base URL; `NMBS_IRAIL_API` overrides it (handy for testing).
fn api_base() -> String {
    std::env::var("NMBS_IRAIL_API").unwrap_or_else(|_| API.to_owned())
}
const MIN_GAP: Duration = Duration::from_millis(350);
const USER_AGENT: &str = concat!(
    "NMBS-Vertrekbord/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/karbos/NMBS-Screen-Simulator)"
);
const INTERNATIONAL_TYPES: &[&str] = &["ICE", "EUR", "EST", "TGV", "THA", "ECD", "ICD", "EC", "NJ"];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub standardname: String,
}

#[derive(Debug, Clone)]
pub struct Liveboard {
    pub station: String,
    pub trains: Vec<Train>,
}

pub enum Request {
    Stations {
        lang: String,
    },
    Liveboard {
        generation: u64,
        station_id: String,
        arrdep: ArrDep,
        lang: String,
    },
    Vehicle {
        key: String,
        vehicle: String,
        date: String,
        station_id: String,
        arrdep: ArrDep,
        lang: String,
    },
}

pub enum Response {
    Stations {
        lang: String,
        result: Result<Vec<Station>, String>,
    },
    Liveboard {
        generation: u64,
        result: Result<Liveboard, String>,
    },
    Via {
        key: String,
        result: Result<Vec<String>, String>,
    },
}

pub struct LiveClient {
    tx: Sender<Request>,
    pub rx: Receiver<Response>,
}

impl LiveClient {
    /// Starts the worker thread. `wake` is called after every response so
    /// the UI can repaint.
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        let (tx, worker_rx) = channel::<Request>();
        let (worker_tx, rx) = channel::<Response>();
        std::thread::Builder::new()
            .name("irail".into())
            .spawn(move || worker(worker_rx, worker_tx, wake))
            .expect("kan iRail-thread niet starten");
        Self { tx, rx }
    }

    pub fn send(&self, req: Request) {
        let _ = self.tx.send(req);
    }
}

fn worker(rx: Receiver<Request>, tx: Sender<Response>, wake: impl Fn()) {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(USER_AGENT)
        .build()
        .into();
    let mut last = Instant::now() - MIN_GAP;
    let base = api_base();
    let mut get = |path: &str, params: &[(&str, &str)]| -> Result<Value, String> {
        let wait = MIN_GAP.saturating_sub(last.elapsed());
        std::thread::sleep(wait);
        last = Instant::now();
        let mut req = agent.get(format!("{base}/{path}/")).query("format", "json");
        for (k, v) in params {
            req = req.query(*k, *v);
        }
        let mut res = req.call().map_err(|e| match e {
            ureq::Error::StatusCode(code) => format!("HTTP {code}"),
            other => other.to_string(),
        })?;
        res.body_mut()
            .with_config()
            .limit(20 * 1024 * 1024)
            .read_json::<Value>()
            .map_err(|e| e.to_string())
    };

    while let Ok(req) = rx.recv() {
        let resp = match req {
            Request::Stations { lang } => {
                let result = get("stations", &[("lang", &lang)]).map(|v| parse_stations(&v));
                Response::Stations { lang, result }
            }
            Request::Liveboard {
                generation,
                station_id,
                arrdep,
                lang,
            } => {
                let result = get(
                    "liveboard",
                    &[
                        ("id", &station_id),
                        ("arrdep", arrdep.api()),
                        ("lang", &lang),
                        ("alerts", "false"),
                    ],
                )
                .map(|v| parse_liveboard(&v, arrdep));
                Response::Liveboard { generation, result }
            }
            Request::Vehicle {
                key,
                vehicle,
                date,
                station_id,
                arrdep,
                lang,
            } => {
                let result = get(
                    "vehicle",
                    &[("id", &vehicle), ("date", &date), ("lang", &lang), ("alerts", "false")],
                )
                .map(|v| parse_via(&v, &station_id, arrdep));
                Response::Via { key, result }
            }
        };
        if tx.send(resp).is_err() {
            break;
        }
        wake();
    }
}

// ---------- Parsing ----------

/// iRail returns most values as strings; accept numbers too.
fn s(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn n(v: &Value) -> i64 {
    match v {
        Value::Number(x) => x.as_i64().unwrap_or(0),
        Value::String(x) => x.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

/// iRail uses an object for a single item and an array for several.
fn as_list(v: &Value) -> Vec<&Value> {
    match v {
        Value::Array(a) => a.iter().collect(),
        Value::Null => vec![],
        other => vec![other],
    }
}

fn parse_stations(v: &Value) -> Vec<Station> {
    let mut list: Vec<Station> = as_list(&v["station"])
        .into_iter()
        .map(|st| Station {
            id: s(&st["id"]),
            name: s(&st["name"]),
            standardname: s(&st["standardname"]),
        })
        .filter(|st| !st.id.is_empty())
        .collect();
    list.sort_by_key(|a| a.name.to_lowercase());
    list
}

/// Unix seconds -> "HH:MM" in Belgian time.
pub fn to_hm(unix: i64) -> String {
    DateTime::from_timestamp(unix, 0)
        .map(|d| d.with_timezone(&Brussels).format("%H:%M").to_string())
        .unwrap_or_default()
}

/// Unix seconds -> "ddmmyy" (iRail date format) in Belgian time.
pub fn to_irail_date(unix: i64) -> String {
    DateTime::from_timestamp(unix, 0)
        .map(|d| d.with_timezone(&Brussels).format("%d%m%y").to_string())
        .unwrap_or_default()
}

/// Belgian stations have ids BE.NMBS.0088xxxxx; anything else is abroad.
fn is_foreign(station_id: &str) -> bool {
    !station_id.is_empty() && !station_id.starts_with("BE.NMBS.0088")
}

fn parse_liveboard(v: &Value, arrdep: ArrDep) -> Liveboard {
    let (block, item) = match arrdep {
        ArrDep::Departure => ("departures", "departure"),
        ArrDep::Arrival => ("arrivals", "arrival"),
    };
    let trains = as_list(&v[block][item])
        .into_iter()
        .map(|d| {
            let vi = &d["vehicleinfo"];
            let mut kind = s(&vi["type"]).to_uppercase();
            if kind.is_empty() {
                kind = s(&vi["shortname"]).split(' ').next().unwrap_or("").to_uppercase();
            }
            let other_id = s(&d["stationinfo"]["id"]);
            let platform = s(&d["platform"]);
            let ts = n(&d["time"]);
            let mut vehicle = s(&d["vehicle"]);
            if vehicle.is_empty() {
                vehicle = s(&vi["name"]);
            }
            Train {
                time: to_hm(ts),
                ts,
                dest: s(&d["station"]),
                platform: if platform == "?" { String::new() } else { platform },
                platform_changed: s(&d["platforminfo"]["normal"]) == "0",
                icon: if INTERNATIONAL_TYPES.contains(&kind.as_str()) || is_foreign(&other_id) {
                    Icon::International
                } else {
                    Icon::None
                },
                kind,
                delay: (n(&d["delay"]) as f64 / 60.0).round() as i64,
                status: if s(&d["canceled"]) == "1" {
                    Status::Cancelled
                } else {
                    Status::None
                },
                left: s(&d["left"]) == "1",
                vehicle,
                ..Default::default()
            }
        })
        .collect();
    let station = match s(&v["stationinfo"]["name"]) {
        name if !name.is_empty() => name,
        _ => s(&v["station"]),
    };
    Liveboard { station, trains }
}

/// Intermediate stops between this station and the destination
/// (departures) or between the origin and this station (arrivals).
fn parse_via(v: &Value, station_id: &str, arrdep: ArrDep) -> Vec<String> {
    let stops = as_list(&v["stops"]["stop"]);
    let Some(idx) = stops.iter().position(|st| s(&st["stationinfo"]["id"]) == station_id) else {
        return vec![];
    };
    let between: &[&Value] = match arrdep {
        ArrDep::Departure if idx + 1 < stops.len() => &stops[idx + 1..stops.len() - 1],
        ArrDep::Arrival if idx >= 1 => &stops[1..idx],
        _ => &[],
    };
    between
        .iter()
        .filter(|st| s(&st["canceled"]) != "1")
        .map(|st| s(&st["station"]))
        .collect()
}

/// Picks at most `max` stops spread evenly over the list.
pub fn spread(list: &[String], max: usize) -> Vec<String> {
    if list.len() <= max {
        return list.to_vec();
    }
    if max == 0 {
        return vec![];
    }
    if max == 1 {
        return vec![list[0].clone()];
    }
    let mut out: Vec<String> = Vec::new();
    for i in 0..max {
        let item = &list[(i as f64 * (list.len() - 1) as f64 / (max - 1) as f64).round() as usize];
        if !out.contains(item) {
            out.push(item.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stop(id: &str, name: &str) -> Value {
        json!({ "station": name, "stationinfo": { "id": id }, "canceled": "0" })
    }

    #[test]
    fn parses_liveboard() {
        let v = json!({
            "station": "Brussels-South",
            "stationinfo": { "id": "BE.NMBS.008814001", "name": "Brussel-Zuid" },
            "departures": { "number": "2", "departure": [
                { "station": "Antwerpen-Centraal", "stationinfo": { "id": "BE.NMBS.008821006" },
                  "time": "1760432400", "delay": "360", "canceled": "0", "left": "0",
                  "vehicle": "BE.NMBS.IC2012", "vehicleinfo": { "type": "IC", "shortname": "IC 2012" },
                  "platform": "13", "platforminfo": { "name": "13", "normal": "1" } },
                { "station": "Amsterdam-Centraal", "stationinfo": { "id": "BE.NMBS.008400058" },
                  "time": "1760432700", "delay": "0", "canceled": "1", "left": "0",
                  "vehicle": "BE.NMBS.EC9227", "vehicleinfo": { "shortname": "EC 9227" },
                  "platform": "3", "platforminfo": { "name": "3", "normal": "0" } }
            ]}
        });
        let b = parse_liveboard(&v, ArrDep::Departure);
        assert_eq!(b.station, "Brussel-Zuid");
        assert_eq!(b.trains.len(), 2);
        assert_eq!(b.trains[0].delay, 6);
        assert_eq!(b.trains[0].kind, "IC");
        assert_eq!(b.trains[0].icon, Icon::None);
        assert_eq!(b.trains[0].time, "11:00");
        assert_eq!(b.trains[1].kind, "EC");
        assert_eq!(b.trains[1].icon, Icon::International);
        assert_eq!(b.trains[1].status, Status::Cancelled);
        assert!(b.trains[1].platform_changed);
    }

    #[test]
    fn single_departure_is_an_object() {
        let v = json!({ "departures": { "departure": { "station": "Gent", "time": "0" } } });
        assert_eq!(parse_liveboard(&v, ArrDep::Departure).trains.len(), 1);
    }

    #[test]
    fn via_stops() {
        let v = json!({ "stops": { "stop": [
            stop("A", "Charleroi"), stop("ME", "Brussel-Zuid"), stop("B", "Brussel-Centraal"),
            stop("C", "Brussel-Noord"), stop("D", "Mechelen"), stop("E", "Antwerpen")
        ]}});
        assert_eq!(
            parse_via(&v, "ME", ArrDep::Departure),
            ["Brussel-Centraal", "Brussel-Noord", "Mechelen"]
        );
        assert_eq!(
            parse_via(&v, "D", ArrDep::Arrival),
            ["Brussel-Zuid", "Brussel-Centraal", "Brussel-Noord"]
        );
        assert!(parse_via(&v, "X", ArrDep::Departure).is_empty());
        assert!(parse_via(&v, "E", ArrDep::Departure).is_empty());
    }

    #[test]
    fn spread_keeps_order() {
        let l: Vec<String> = (1..=7).map(|i| i.to_string()).collect();
        assert_eq!(spread(&l, 3), ["1", "4", "7"]);
        assert_eq!(spread(&l[..2], 3), ["1", "2"]);
    }
}
