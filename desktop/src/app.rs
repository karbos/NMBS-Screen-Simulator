//! Application state, live-data handling, persistence and the main frame.

use crate::board::{self, BoardView};
use crate::clock::Clock;
use crate::live::{self, LiveClient, Request, Response, Station};
use crate::model::{self, SaveData, Source, Train};
use egui::{Key, ViewportCommand};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const STATION_CACHE_TTL: u64 = 7 * 24 * 3600;
const SAVE_DELAY: Duration = Duration::from_millis(800);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Trains,
    Live,
    General,
    Clock,
    Colors,
    Data,
}

pub struct LiveState {
    pub client: LiveClient,
    pub generation: u64,
    pub status: String,
    pub status_is_error: bool,
    pub next_refresh: Option<Instant>,
    pub in_flight: bool,
    /// Signature of the settings that require a reload when changed.
    pub key: String,
    pub stations: Vec<Station>,
    pub stations_lang: String,
    pub stations_pending: bool,
    pub station_query: String,
    pub via_cache: HashMap<String, Vec<String>>,
    pub via_pending: HashSet<String>,
}

pub struct BoardApp {
    pub data: SaveData,
    pub clock: Clock,
    pub panel_open: bool,
    pub tab: Tab,
    pub live: LiveState,
    pub confirm_reset: bool,
    pub confirm_clear: bool,
    pub message: Option<String>,
    config_path: PathBuf,
    dirty_since: Option<Instant>,
    last_sim: Instant,
    start: Instant,
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nmbs-vertrekbord")
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("nmbs-vertrekbord")
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl BoardApp {
    pub fn new(ctx: &egui::Context, show_panel: bool) -> Self {
        let config_path = config_dir().join("config.json");
        let data = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| serde_json::from_str::<SaveData>(&s).ok())
            .unwrap_or_else(SaveData::defaults);
        let wake_ctx = ctx.clone();
        let client = LiveClient::start(move || wake_ctx.request_repaint());
        let clock = Clock::new(&data.settings);
        let station_query = data.settings.live_station_name.clone();
        Self {
            clock,
            panel_open: show_panel,
            tab: Tab::Trains,
            live: LiveState {
                client,
                generation: 0,
                status: String::new(),
                status_is_error: false,
                next_refresh: None,
                in_flight: false,
                key: String::new(),
                stations: Vec::new(),
                stations_lang: String::new(),
                stations_pending: false,
                station_query,
                via_cache: HashMap::new(),
                via_pending: HashSet::new(),
            },
            confirm_reset: false,
            confirm_clear: false,
            message: None,
            config_path,
            dirty_since: None,
            last_sim: Instant::now(),
            start: Instant::now(),
            data,
        }
    }

    // ---------- Persistence ----------

    pub fn mark_dirty(&mut self) {
        self.dirty_since.get_or_insert_with(Instant::now);
    }

    pub fn save_now(&mut self) {
        self.dirty_since = None;
        if let Some(dir) = self.config_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.data) {
            let _ = std::fs::write(&self.config_path, json);
        }
    }

    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Replaces all data (import / reset) and restarts clock and live data.
    pub fn replace_data(&mut self, data: SaveData) {
        self.data = data;
        self.clock.restart(&self.data.settings);
        self.live.station_query = self.data.settings.live_station_name.clone();
        self.live.key.clear();
        self.mark_dirty();
    }

    // ---------- Helpers ----------

    pub fn now_minutes(&self) -> f64 {
        self.clock.now_minutes(&self.data.settings)
    }

    /// Language currently shown (cycles when configured).
    fn current_lang(&self) -> &'static str {
        let cycle = self.data.settings.language.cycle();
        let interval = self.data.settings.cycle_interval.max(2.0);
        cycle[(unix_now() as f64 / interval) as usize % cycle.len()]
    }

    pub fn add_random_train(&mut self) {
        let now = self.now_minutes();
        let at = model::next_free_time(&self.data.trains, now);
        self.data.trains.push(model::random_train(at));
        if self.data.settings.auto_sort {
            model::sort_trains(&mut self.data.trains, now);
        }
        self.mark_dirty();
    }

    pub fn add_train(&mut self) {
        let at = model::next_free_time(&self.data.trains, self.now_minutes());
        self.data
            .trains
            .push(Train::new(&model::fmt_hm(at), "Nieuwe bestemming", "", "1", "IC"));
        self.mark_dirty();
    }

    pub fn set_live_status(&mut self, text: impl Into<String>, error: bool) {
        self.live.status = text.into();
        self.live.status_is_error = error;
    }

    pub fn select_station(&mut self, st: &Station) {
        self.data.settings.live_station_id = st.id.clone();
        self.data.settings.live_station_name = st.name.clone();
        self.live.station_query = st.name.clone();
        self.data.trains.clear();
        self.mark_dirty();
    }

    pub fn refresh_live_now(&mut self) {
        self.live.next_refresh = Some(Instant::now());
    }

    pub fn station_matches(&self, query: &str, max: usize) -> Vec<Station> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return vec![];
        }
        let starts = self
            .live
            .stations
            .iter()
            .filter(|s| s.name.to_lowercase().starts_with(&q));
        let contains = self.live.stations.iter().filter(|s| {
            let n = s.name.to_lowercase();
            !n.starts_with(&q) && (n.contains(&q) || s.standardname.to_lowercase().contains(&q))
        });
        starts.chain(contains).take(max).cloned().collect()
    }

    // ---------- Live data ----------

    fn live_key(&self) -> String {
        let s = &self.data.settings;
        format!(
            "{:?}|{}|{:?}|{}|{}|{}|{}",
            s.source,
            s.live_station_id,
            s.live_arrdep,
            s.language.primary(),
            s.live_platforms,
            s.live_hide_left,
            s.live_via
        )
    }

    fn load_stations(&mut self) {
        let lang = self.data.settings.language.primary();
        if self.live.stations_lang == lang || self.live.stations_pending {
            return;
        }
        let cache_file = cache_dir().join(format!("stations-{lang}.json"));
        if let Ok(text) = std::fs::read_to_string(&cache_file)
            && let Ok((at, list)) = serde_json::from_str::<(u64, Vec<Station>)>(&text)
            && unix_now().saturating_sub(at) < STATION_CACHE_TTL
            && !list.is_empty()
        {
            self.live.stations = list;
            self.live.stations_lang = lang.to_owned();
            return;
        }
        self.live.stations_pending = true;
        self.live.client.send(Request::Stations { lang: lang.to_owned() });
    }

    fn update_live(&mut self) {
        // React to changed live settings.
        let key = self.live_key();
        if key != self.live.key {
            let old = std::mem::replace(&mut self.live.key, key);
            if self.data.settings.source == Source::Live {
                // Via lines depend on station, language and count: look them up again.
                for t in &mut self.data.trains {
                    t.via.clear();
                }
                self.load_stations();
                let delay = if old.is_empty() || old.starts_with("Manual") {
                    0
                } else {
                    600
                };
                self.live.next_refresh = Some(Instant::now() + Duration::from_millis(delay));
            } else {
                self.live.generation += 1;
                self.live.next_refresh = None;
                self.live.in_flight = false;
                self.set_live_status("Live gegevens staan uit.", false);
            }
        }

        // Handle responses.
        while let Ok(resp) = self.live.client.rx.try_recv() {
            match resp {
                Response::Stations { lang, result } => {
                    self.live.stations_pending = false;
                    match result {
                        Ok(list) => {
                            let _ = std::fs::create_dir_all(cache_dir());
                            if let Ok(json) = serde_json::to_string(&(unix_now(), &list)) {
                                let _ = std::fs::write(cache_dir().join(format!("stations-{lang}.json")), json);
                            }
                            self.live.stations = list;
                            self.live.stations_lang = lang;
                        }
                        Err(e) => self.set_live_status(format!("Stationslijst kon niet geladen worden: {e}"), true),
                    }
                }
                Response::Liveboard { generation, result } => {
                    if generation != self.live.generation {
                        continue;
                    }
                    self.live.in_flight = false;
                    let refresh = Duration::from_secs_f64(self.data.settings.live_refresh.max(30.0));
                    self.live.next_refresh = Some(Instant::now() + refresh);
                    match result {
                        Ok(board) => self.apply_liveboard(board),
                        Err(e) => self.set_live_status(format!("Fout bij ophalen live gegevens: {e}"), true),
                    }
                }
                Response::Via { key, result } => {
                    self.live.via_pending.remove(&key);
                    // Errors are cached as "no via" so they are not retried every frame.
                    self.live.via_cache.insert(key, result.unwrap_or_default());
                }
            }
        }

        if self.data.settings.source != Source::Live {
            return;
        }

        // Periodic refresh.
        if !self.live.in_flight && self.live.next_refresh.is_some_and(|t| Instant::now() >= t) {
            self.live.generation += 1;
            self.live.in_flight = true;
            self.live.next_refresh = None;
            self.set_live_status("Live gegevens ophalen…", false);
            let s = &self.data.settings;
            self.live.client.send(Request::Liveboard {
                generation: self.live.generation,
                station_id: s.live_station_id.clone(),
                arrdep: s.live_arrdep,
                lang: s.language.primary().to_owned(),
            });
        }

        // Via stations for the visible rows.
        let s = &self.data.settings;
        let max = s.live_via as usize;
        if max == 0 {
            return;
        }
        let rows = s.rows();
        for t in self.data.trains.iter_mut().take(rows) {
            if !t.via.is_empty() || t.vehicle.is_empty() {
                continue;
            }
            let date = live::to_irail_date(t.ts);
            let lang = s.language.primary();
            let key = format!(
                "{}|{}|{}|{:?}|{}",
                t.vehicle, date, s.live_station_id, s.live_arrdep, lang
            );
            if let Some(list) = self.live.via_cache.get(&key) {
                t.via = live::spread(list, max).join(", ");
            } else if self.live.via_pending.insert(key.clone()) {
                self.live.client.send(Request::Vehicle {
                    key,
                    vehicle: t.vehicle.clone(),
                    date,
                    station_id: s.live_station_id.clone(),
                    arrdep: s.live_arrdep,
                    lang: lang.to_owned(),
                });
            }
        }
    }

    fn apply_liveboard(&mut self, board: live::Liveboard) {
        let s = &self.data.settings;
        let platforms: Vec<String> = s
            .live_platforms
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .map(|p| p.trim().to_uppercase())
            .filter(|p| !p.is_empty())
            .collect();
        self.data.trains = board
            .trains
            .into_iter()
            .filter(|t| !(s.live_hide_left && t.left))
            .filter(|t| platforms.is_empty() || platforms.contains(&t.platform.to_uppercase()))
            .collect();
        if !board.station.is_empty() {
            self.data.settings.live_station_name = board.station.clone();
            self.live.station_query = board.station.clone();
        }
        let stamp = chrono::Utc::now()
            .with_timezone(&chrono_tz::Europe::Brussels)
            .format("%H:%M:%S");
        let count = self.data.trains.len();
        self.set_live_status(
            format!("{}: {count} trein(en) · bijgewerkt om {stamp}", board.station),
            false,
        );
        self.mark_dirty();
    }

    // ---------- Keyboard ----------

    fn handle_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (toggle, full, random, esc) = ctx.input(|i| {
            let plain = !i.modifiers.ctrl && !i.modifiers.command && !i.modifiers.alt;
            (
                plain && (i.key_pressed(Key::P) || i.key_pressed(Key::H)),
                plain && (i.key_pressed(Key::F) || i.key_pressed(Key::F11)),
                plain && i.key_pressed(Key::R),
                i.key_pressed(Key::Escape),
            )
        });
        if toggle {
            self.panel_open = !self.panel_open;
        }
        if full {
            let is_full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!is_full));
        }
        if random {
            self.add_random_train();
        }
        if esc {
            self.panel_open = false;
        }
    }
}

impl eframe::App for BoardApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_live();

        if self.last_sim.elapsed() >= Duration::from_millis(250) {
            self.last_sim = Instant::now();
            let now = self.now_minutes();
            if model::simulate(&mut self.data.trains, &self.data.settings, now) {
                self.mark_dirty();
            }
        }

        if self.dirty_since.is_some_and(|t| t.elapsed() >= SAVE_DELAY) {
            self.save_now();
        }
        self.handle_keys(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.panel_open {
            egui::Panel::right("settings")
                .resizable(true)
                .default_size(440.0)
                .size_range(320.0..=720.0)
                .show(ui, |ui| self.panel_ui(ui));
        }

        let mut animating = false;
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let rect = ui.max_rect();
            let resp = ui.allocate_rect(rect, egui::Sense::click());
            if resp.double_clicked() {
                let is_full = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ui.ctx().send_viewport_cmd(ViewportCommand::Fullscreen(!is_full));
            }
            let lang = self.current_lang();
            let view = BoardView {
                settings: &self.data.settings,
                trains: &self.data.trains,
                texts: model::texts(lang),
                now_sec: self.clock.now_seconds(&self.data.settings),
                anim_time: self.start.elapsed().as_secs_f64(),
            };
            animating = board::paint(ui.painter(), rect, &view);
        });

        if !self.panel_open {
            // Small button to reopen the panel; hide the cursor when idle.
            let idle = ui.input(|i| i.pointer.time_since_last_movement() > 3.0);
            if idle {
                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
            } else {
                egui::Area::new("open_panel".into())
                    .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
                    .show(ui.ctx(), |ui| {
                        if ui.button("⚙ Instellingen (P)").clicked() {
                            self.panel_open = true;
                        }
                    });
            }
        }

        let next = if animating {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(250)
        };
        ui.ctx().request_repaint_after(next);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_now();
    }
}
