//! Data model and simulation logic of the departure board.
//!
//! The JSON layout of [`SaveData`] is identical to the export of the web
//! version, so configurations can be shared between both.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------- Enums ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[serde(rename = "fr")]
    Fr,
    #[serde(rename = "de")]
    De,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "nl-fr")]
    NlFr,
    #[serde(rename = "nl-fr-de-en")]
    All,
    #[default]
    #[serde(rename = "nl", other)]
    Nl,
}

impl Language {
    pub const ALL: [Language; 6] = [
        Language::Nl,
        Language::Fr,
        Language::De,
        Language::En,
        Language::NlFr,
        Language::All,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Language::Nl => "Nederlands",
            Language::Fr => "Français",
            Language::De => "Deutsch",
            Language::En => "English",
            Language::NlFr => "Wisselen NL / FR",
            Language::All => "Wisselen NL / FR / DE / EN",
        }
    }

    /// The languages this setting cycles through (one for a fixed language).
    pub fn cycle(self) -> &'static [&'static str] {
        match self {
            Language::Nl => &["nl"],
            Language::Fr => &["fr"],
            Language::De => &["de"],
            Language::En => &["en"],
            Language::NlFr => &["nl", "fr"],
            Language::All => &["nl", "fr", "de", "en"],
        }
    }

    /// Language used for live data (station names): the first of the cycle.
    pub fn primary(self) -> &'static str {
        self.cycle()[0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClockMode {
    Offset,
    Custom,
    #[default]
    #[serde(other)]
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Live,
    #[default]
    #[serde(other)]
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ArrDep {
    Arrival,
    #[default]
    #[serde(other)]
    Departure,
}

impl ArrDep {
    pub fn api(self) -> &'static str {
        match self {
            ArrDep::Departure => "departure",
            ArrDep::Arrival => "arrival",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Status {
    #[serde(rename = "arriving")]
    Arriving,
    #[serde(rename = "limited")]
    Limited,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "custom")]
    Custom,
    #[default]
    #[serde(rename = "", other)]
    None,
}

impl Status {
    pub const ALL: [Status; 5] = [
        Status::None,
        Status::Arriving,
        Status::Limited,
        Status::Cancelled,
        Status::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Status::None => "Geen",
            Status::Arriving => "Komt aan",
            Status::Limited => "Beperkt",
            Status::Cancelled => "Afgeschaft",
            Status::Custom => "Eigen tekst…",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Icon {
    #[serde(rename = "int")]
    International,
    #[default]
    #[serde(rename = "", other)]
    None,
}

// ---------- Colours ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Colors {
    pub header: String,
    pub row_a: String,
    pub row_b: String,
    pub separator: String,
    pub dest: String,
    pub text: String,
    pub alert: String,
    pub info: String,
    pub new_time_bg: String,
    pub new_time_text: String,
    pub platform_change: String,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            header: "#1560bd".into(),
            row_a: "#12356a".into(),
            row_b: "#163b73".into(),
            separator: "#2c5a95".into(),
            dest: "#ffd400".into(),
            text: "#ffffff".into(),
            alert: "#e2231a".into(),
            info: "#1673d6".into(),
            new_time_bg: "#ffffff".into(),
            new_time_text: "#0b2550".into(),
            platform_change: "#ffd400".into(),
        }
    }
}

impl Colors {
    /// (label, field) pairs for the colour editor.
    pub fn fields_mut(&mut self) -> [(&'static str, &mut String); 11] {
        [
            ("Kopbalk", &mut self.header),
            ("Rij (oneven)", &mut self.row_a),
            ("Rij (even)", &mut self.row_b),
            ("Scheidingslijn", &mut self.separator),
            ("Bestemming", &mut self.dest),
            ("Tekst", &mut self.text),
            ("Vertraging / beperkt", &mut self.alert),
            ("Komt aan / info", &mut self.info),
            ("Nieuwe tijd vak", &mut self.new_time_bg),
            ("Nieuwe tijd tekst", &mut self.new_time_text),
            ("Spoorwijziging", &mut self.platform_change),
        ]
    }
}

/// Parses "#rrggbb" (or "rrggbb"); falls back to magenta so mistakes are visible.
pub fn parse_hex(s: &str) -> [u8; 3] {
    let h = s.trim().trim_start_matches('#');
    if h.len() == 6
        && let Ok(v) = u32::from_str_radix(h, 16)
    {
        return [(v >> 16) as u8, (v >> 8) as u8, v as u8];
    }
    [255, 0, 255]
}

pub fn to_hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

// ---------- Settings ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub title: String,
    pub language: Language,
    pub cycle_interval: f64,
    pub rows_visible: u32,
    pub show_logo: bool,
    pub show_seconds: bool,
    pub blink_colon: bool,
    pub marquee: bool,
    pub show_via_word: bool,
    pub alternate_rows: bool,
    pub show_empty_rows: bool,
    pub clock_mode: ClockMode,
    pub clock_offset: f64,
    pub custom_start: String,
    pub clock_speed: f64,
    pub auto_depart: bool,
    pub depart_grace: f64,
    pub auto_fill: bool,
    pub auto_sort: bool,
    pub auto_arriving: f64,
    pub source: Source,
    pub live_station_id: String,
    pub live_station_name: String,
    pub live_arrdep: ArrDep,
    pub live_refresh: f64,
    pub live_via: u32,
    pub live_platforms: String,
    pub live_hide_left: bool,
    pub live_show_station: bool,
    pub colors: Colors,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: String::new(),
            language: Language::Nl,
            cycle_interval: 8.0,
            rows_visible: 8,
            show_logo: true,
            show_seconds: false,
            blink_colon: false,
            marquee: true,
            show_via_word: true,
            alternate_rows: false,
            show_empty_rows: true,
            clock_mode: ClockMode::Live,
            clock_offset: 0.0,
            custom_start: "17:20:00".into(),
            clock_speed: 1.0,
            auto_depart: false,
            depart_grace: 1.0,
            auto_fill: false,
            auto_sort: false,
            auto_arriving: 0.0,
            source: Source::Manual,
            live_station_id: "BE.NMBS.008814001".into(),
            live_station_name: "Brussel-Zuid".into(),
            live_arrdep: ArrDep::Departure,
            live_refresh: 60.0,
            live_via: 3,
            live_platforms: String::new(),
            live_hide_left: true,
            live_show_station: false,
            colors: Colors::default(),
        }
    }
}

impl Settings {
    pub fn rows(&self) -> usize {
        self.rows_visible.clamp(3, 14) as usize
    }
}

// ---------- Trains ----------

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn new_id() -> u64 {
    next_id()
}

fn default_status_color() -> String {
    "#1673d6".into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Train {
    #[serde(skip, default = "new_id")]
    pub id: u64,
    pub time: String,
    pub dest: String,
    pub via: String,
    pub remark: String,
    #[serde(deserialize_with = "string_or_number")]
    pub platform: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub icon: Icon,
    pub delay: i64,
    pub expected: String,
    pub status: Status,
    pub custom_status: String,
    #[serde(default = "default_status_color")]
    pub custom_color: String,
    pub platform_changed: bool,
    /// iRail vehicle id (live data only).
    pub vehicle: String,
    /// Unix time of the scheduled departure (live data only).
    pub ts: i64,
    /// Train has already left (live data only, not saved).
    #[serde(skip)]
    pub left: bool,
}

impl Default for Train {
    fn default() -> Self {
        Self {
            id: new_id(),
            time: "12:00".into(),
            dest: String::new(),
            via: String::new(),
            remark: String::new(),
            platform: String::new(),
            kind: String::new(),
            icon: Icon::None,
            delay: 0,
            expected: String::new(),
            status: Status::None,
            custom_status: String::new(),
            custom_color: default_status_color(),
            platform_changed: false,
            vehicle: String::new(),
            ts: 0,
            left: false,
        }
    }
}

fn string_or_number<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    })
}

impl Train {
    pub fn new(time: &str, dest: &str, via: &str, platform: &str, kind: &str) -> Self {
        Self {
            time: time.into(),
            dest: dest.into(),
            via: via.into(),
            platform: platform.into(),
            kind: kind.into(),
            ..Default::default()
        }
    }

    /// Copy with a fresh id.
    pub fn duplicate(&self) -> Self {
        Self {
            id: new_id(),
            ..self.clone()
        }
    }

    pub fn scheduled_min(&self) -> f64 {
        parse_hm(&self.time).unwrap_or(0.0)
    }

    /// Expected departure: the explicit expected time, else time + delay.
    pub fn expected_min(&self) -> f64 {
        parse_hm(&self.expected).unwrap_or_else(|| self.scheduled_min() + self.delay as f64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SaveData {
    pub settings: Settings,
    pub trains: Vec<Train>,
}

impl SaveData {
    pub fn defaults() -> Self {
        Self {
            settings: Settings::default(),
            trains: default_trains(),
        }
    }
}

pub fn default_trains() -> Vec<Train> {
    vec![
        Train {
            delay: 6,
            ..Train::new("17:17", "Quévry", "Halle, ‘s-Gravenbrakel, Bergen", "19", "P")
        },
        Train::new("17:17", "Gent-Sint-Pieters", "Denderleeuw, Aalst, Wetteren", "7", "IC"),
        Train {
            delay: 90,
            expected: "18:55".into(),
            ..Train::new("17:17", "Antwerpen-Centraal", "Mechelen", "13", "IC")
        },
        Train {
            status: Status::Arriving,
            ..Train::new("17:25", "Louvain-la-Neuve", "Brussel-Schuman, Etterbeek", "18", "S8")
        },
        Train {
            icon: Icon::International,
            ..Train::new("17:25", "Dortmund Hbf", "", "3", "ICE")
        },
        Train::new(
            "17:25",
            "Binche",
            "Halle, ‘s-Gravenbrakel, La Louvière-Zuid",
            "21",
            "IC",
        ),
        Train {
            remark: "Rijdt niet verder dan Brugge".into(),
            status: Status::Limited,
            ..Train::new("17:25", "Oostende", "", "15", "IC")
        },
        Train::new(
            "17:33",
            "Nijvel",
            "Sint-Genesius-Rode, Waterloo, Eigenbrakel",
            "4",
            "S1",
        ),
    ]
}

// ---------- Texts ----------

pub struct LangTexts {
    pub title: &'static str,
    pub arr_title: &'static str,
    pub arriving: &'static str,
    pub limited: &'static str,
    pub cancelled: &'static str,
    pub via: &'static str,
}

pub fn texts(lang: &str) -> &'static LangTexts {
    static NL: LangTexts = LangTexts {
        title: "Vertrek",
        arr_title: "Aankomst",
        arriving: "Komt aan",
        limited: "Beperkt",
        cancelled: "Afgeschaft",
        via: "via",
    };
    static FR: LangTexts = LangTexts {
        title: "Départ",
        arr_title: "Arrivée",
        arriving: "Arrive",
        limited: "Limité",
        cancelled: "Supprimé",
        via: "via",
    };
    static DE: LangTexts = LangTexts {
        title: "Abfahrt",
        arr_title: "Ankunft",
        arriving: "Kommt an",
        limited: "Begrenzt",
        cancelled: "Fällt aus",
        via: "über",
    };
    static EN: LangTexts = LangTexts {
        title: "Departure",
        arr_title: "Arrival",
        arriving: "Arriving",
        limited: "Limited",
        cancelled: "Cancelled",
        via: "via",
    };
    match lang {
        "fr" => &FR,
        "de" => &DE,
        "en" => &EN,
        _ => &NL,
    }
}

struct Route {
    dest: &'static str,
    via: &'static str,
    types: &'static [&'static str],
    international: bool,
}

const fn r(dest: &'static str, via: &'static str, types: &'static [&'static str]) -> Route {
    Route {
        dest,
        via,
        types,
        international: false,
    }
}

const fn ri(dest: &'static str, via: &'static str, types: &'static [&'static str]) -> Route {
    Route {
        dest,
        via,
        types,
        international: true,
    }
}

const ROUTES: &[Route] = &[
    r("Quévry", "Halle, ‘s-Gravenbrakel, Bergen", &["P", "IC"]),
    r("Gent-Sint-Pieters", "Denderleeuw, Aalst, Wetteren", &["IC"]),
    r("Antwerpen-Centraal", "Mechelen", &["IC"]),
    r("Louvain-la-Neuve", "Brussel-Schuman, Etterbeek", &["S8"]),
    ri("Dortmund Hbf", "", &["ICE"]),
    ri(
        "Frankfurt (Main) Hbf",
        "Liège-Guillemins, Aachen Hbf, Köln Hbf",
        &["ICE"],
    ),
    ri("Paris-Nord", "", &["EUR"]),
    ri("Amsterdam-Centraal", "Antwerpen-Centraal, Rotterdam Centraal", &["EC"]),
    r("Binche", "Halle, ‘s-Gravenbrakel, La Louvière-Zuid", &["IC"]),
    r("Oostende", "Gent-Sint-Pieters, Brugge", &["IC"]),
    r("Nijvel", "Sint-Genesius-Rode, Waterloo, Eigenbrakel", &["S1"]),
    r("Luik-Guillemins", "Leuven, Tienen, Ans", &["IC"]),
    r("Hasselt", "Leuven, Aarschot, Diest", &["IC"]),
    r("Namen", "Brussel-Luxemburg, Ottignies, Gembloers", &["IC"]),
    r("Charleroi-Centraal", "Nijvel, Luttre", &["IC"]),
    r("Kortrijk", "Gent-Sint-Pieters, Deinze, Waregem", &["IC"]),
    r("Brugge", "Gent-Sint-Pieters", &["IC"]),
    r("Knokke", "Gent-Sint-Pieters, Brugge", &["IC"]),
    r("Blankenberge", "Gent-Sint-Pieters, Brugge", &["IC"]),
    r("Brussels Airport - Zaventem", "Brussel-Noord", &["IC", "S9"]),
    r("Mechelen", "Brussel-Noord, Vilvoorde", &["S1", "L"]),
    r("Leuven", "Brussel-Noord, Zaventem, Kortenberg", &["S2", "L"]),
    r("Dendermonde", "Jette, Asse, Londerzeel", &["S10"]),
    r("Geraardsbergen", "Halle, Edingen", &["S6", "L"]),
    r("Aalst", "Jette, Dilbeek, Denderleeuw", &["S10"]),
    r("Ottignies", "Etterbeek, Watermaal, Genval", &["S8"]),
    r("Eupen", "Leuven, Luik-Guillemins, Verviers-Centraal", &["IC"]),
    r("Turnhout", "Mechelen, Lier, Herentals", &["IC"]),
    r("Sint-Niklaas", "Dendermonde", &["IC"]),
    ri("Luxembourg", "Namen, Ciney, Libramont, Arlon", &["IC"]),
    ri("Lille-Flandres", "Halle, Doornik", &["IC"]),
];

// ---------- Time helpers ----------

/// "HH:MM" or "HH:MM:SS" -> minutes of the day.
pub fn parse_hm(s: &str) -> Option<f64> {
    let mut parts = s.trim().split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m_str = parts.next()?;
    if m_str.len() != 2 {
        return None;
    }
    let m: u32 = m_str.parse().ok()?;
    let sec: u32 = match parts.next() {
        Some(x) => x.parse().ok()?,
        None => 0,
    };
    if parts.next().is_some() || h > 23 || m > 59 || sec > 59 {
        return None;
    }
    Some(h as f64 * 60.0 + m as f64 + sec as f64 / 60.0)
}

pub fn fmt_hm(min: f64) -> String {
    let total = (min.round() as i64).rem_euclid(1440);
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// Minutes from `now` until `target`, normalised to [-720, 720).
pub fn minutes_until(target: f64, now: f64) -> f64 {
    let mut d = (target - now).rem_euclid(1440.0);
    if d >= 720.0 {
        d -= 1440.0;
    }
    d
}

// ---------- Simulation ----------

pub fn sort_trains(trains: &mut [Train], now: f64) {
    trains.sort_by(|a, b| minutes_until(a.scheduled_min(), now).total_cmp(&minutes_until(b.scheduled_min(), now)));
}

pub fn random_train(after_min: f64) -> Train {
    let route = &ROUTES[fastrand::usize(..ROUTES.len())];
    let mut t = Train::new(
        &fmt_hm(after_min),
        route.dest,
        route.via,
        &(1 + fastrand::u32(..22)).to_string(),
        route.types[fastrand::usize(..route.types.len())],
    );
    if route.international {
        t.icon = Icon::International;
    }
    let roll = fastrand::f64();
    if roll < 0.18 {
        const DELAYS: [i64; 8] = [2, 3, 5, 6, 8, 10, 15, 20];
        t.delay = DELAYS[fastrand::usize(..DELAYS.len())];
    } else if roll < 0.21 {
        t.status = Status::Cancelled;
    } else if roll < 0.23 {
        t.platform_changed = true;
    }
    t
}

/// A departure time a little after the last train on the board.
pub fn next_free_time(trains: &[Train], now: f64) -> f64 {
    let last = trains
        .iter()
        .map(|t| minutes_until(t.scheduled_min(), now))
        .fold(2.0_f64, f64::max);
    (now + last + fastrand::u32(..7) as f64).ceil()
}

/// Shifts all times so the first train leaves in about three minutes.
pub fn shift_to_now(trains: &mut [Train], now: f64) {
    let Some(first) = trains
        .iter()
        .map(|t| minutes_until(t.scheduled_min(), now))
        .reduce(f64::min)
    else {
        return;
    };
    let delta = (3.0 - first).ceil();
    for t in trains.iter_mut() {
        t.time = fmt_hm(t.scheduled_min() + delta);
        if let Some(e) = parse_hm(&t.expected) {
            t.expected = fmt_hm(e + delta);
        }
    }
}

/// Auto-depart, auto-fill and auto-sort. Returns true if the list changed.
pub fn simulate(trains: &mut Vec<Train>, s: &Settings, now: f64) -> bool {
    let mut changed = false;
    if s.auto_depart {
        let before = trains.len();
        trains.retain(|t| {
            let reference = if t.status == Status::Cancelled {
                t.scheduled_min()
            } else {
                t.expected_min()
            };
            -minutes_until(reference, now) < s.depart_grace
        });
        changed |= trains.len() != before;
    }
    if s.auto_fill && s.source == Source::Manual {
        let mut guard = 0;
        while trains.len() < s.rows() && guard < 20 {
            let at = next_free_time(trains, now);
            trains.push(random_train(at));
            changed = true;
            guard += 1;
        }
    }
    if s.auto_sort {
        let before: Vec<u64> = trains.iter().map(|t| t.id).collect();
        sort_trains(trains, now);
        changed |= trains.iter().map(|t| t.id).ne(before.iter().copied());
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_parsing() {
        assert_eq!(parse_hm("17:17"), Some(1037.0));
        assert_eq!(parse_hm("7:05"), Some(425.0));
        assert_eq!(parse_hm("17:20:30"), Some(1040.5));
        assert_eq!(parse_hm("24:00"), None);
        assert_eq!(parse_hm("12:5"), None);
        assert_eq!(fmt_hm(1037.0 + 90.0), "18:47");
        assert_eq!(fmt_hm(1440.0 + 5.0), "00:05");
    }

    #[test]
    fn minutes_until_wraps_midnight() {
        assert_eq!(minutes_until(5.0, 1435.0), 10.0);
        assert_eq!(minutes_until(1435.0, 5.0), -10.0);
    }

    #[test]
    fn web_export_roundtrip() {
        let json = r##"{"settings":{"language":"nl-fr","rowsVisible":10,"clockMode":"custom",
            "colors":{"header":"#000000"}},
            "trains":[{"time":"17:17","dest":"Quévry","platform":19,"type":"P","delay":6,
            "status":"limited","icon":"int"}]}"##;
        let d: SaveData = serde_json::from_str(json).unwrap();
        assert_eq!(d.settings.language, Language::NlFr);
        assert_eq!(d.settings.rows_visible, 10);
        assert_eq!(d.settings.clock_mode, ClockMode::Custom);
        assert_eq!(d.settings.colors.header, "#000000");
        assert_eq!(d.settings.colors.dest, "#ffd400");
        let t = &d.trains[0];
        assert_eq!(t.platform, "19");
        assert_eq!(t.kind, "P");
        assert_eq!(t.status, Status::Limited);
        assert_eq!(t.icon, Icon::International);
        let back = serde_json::to_string(&d).unwrap();
        assert!(back.contains("\"rowsVisible\":10"));
        assert!(back.contains("\"type\":\"P\""));
    }

    #[test]
    fn auto_depart_removes_old_trains() {
        let mut trains = vec![
            Train::new("10:00", "A", "", "1", "IC"),
            Train::new("10:30", "B", "", "2", "IC"),
        ];
        let s = Settings {
            auto_depart: true,
            ..Settings::default()
        };
        assert!(simulate(&mut trains, &s, 10.0 * 60.0 + 5.0));
        assert_eq!(trains.len(), 1);
        assert_eq!(trains[0].dest, "B");
    }
}
