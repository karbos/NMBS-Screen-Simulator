//! The settings panel on the right.

use crate::app::{BoardApp, Tab};
use crate::model::{
    self, ArrDep, ClockMode, Icon, Language, SaveData, Source, Status, Train, parse_hex, parse_hm, to_hex,
};
use egui::{Color32, ComboBox, DragValue, RichText, TextEdit, Ui};

enum TrainAction {
    Up(usize),
    Down(usize),
    Duplicate(usize),
    Delete(usize),
}

const ERROR_RED: Color32 = Color32::from_rgb(200, 40, 30);

/// A small caption above a widget.
fn labeled<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(12.0).weak());
        add(ui)
    })
    .inner
}

/// Single-line text field that turns red when `valid` says so.
fn text_field(ui: &mut Ui, value: &mut String, width: f32, hint: &str, valid: bool) -> bool {
    let mut edit = TextEdit::singleline(value).desired_width(width).hint_text(hint);
    if !valid {
        edit = edit.text_color(ERROR_RED);
    }
    ui.add(edit).changed()
}

fn time_valid(s: &str) -> bool {
    s.is_empty() || parse_hm(s).is_some()
}

impl BoardApp {
    pub fn panel_ui(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.heading("Vertrekbord instellingen");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("×").size(18.0))
                    .on_hover_text("Paneel sluiten (P)")
                    .clicked()
                {
                    self.panel_open = false;
                }
                if ui.button("Volledig scherm").on_hover_text("Toets F of F11").clicked() {
                    let is_full = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_full));
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in [
                (Tab::Trains, "Treinen"),
                (Tab::Live, "Live"),
                (Tab::General, "Algemeen"),
                (Tab::Clock, "Klok & auto"),
                (Tab::Colors, "Kleuren"),
                (Tab::Data, "Data"),
            ] {
                ui.selectable_value(&mut self.tab, tab, label);
            }
        });
        ui.separator();

        let mut changed = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            changed = match self.tab {
                Tab::Trains => self.trains_tab(ui),
                Tab::Live => self.live_tab(ui),
                Tab::General => self.general_tab(ui),
                Tab::Clock => self.clock_tab(ui),
                Tab::Colors => self.colors_tab(ui),
                Tab::Data => self.data_tab(ui),
            };
            ui.add_space(30.0);
        });
        if changed {
            self.mark_dirty();
        }
    }

    // ---------- Trains ----------

    fn trains_tab(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            if ui.button("+ Trein").clicked() {
                self.add_train();
            }
            if ui.button("🎲 Willekeurig").on_hover_text("Toets R").clicked() {
                self.add_random_train();
            }
            if ui.button("Sorteer op tijd").clicked() {
                let now = self.now_minutes();
                model::sort_trains(&mut self.data.trains, now);
                changed = true;
            }
            if ui
                .button("Tijden → nu")
                .on_hover_text("Verschuif alle tijden zodat de eerste trein binnen enkele minuten vertrekt")
                .clicked()
            {
                let now = self.now_minutes();
                model::shift_to_now(&mut self.data.trains, now);
                model::sort_trains(&mut self.data.trains, now);
                changed = true;
            }
            if self.confirm_clear {
                if ui
                    .button(RichText::new("Zeker? Alles wissen").color(ERROR_RED))
                    .clicked()
                {
                    self.data.trains.clear();
                    self.confirm_clear = false;
                    changed = true;
                }
                if ui.button("Annuleren").clicked() {
                    self.confirm_clear = false;
                }
            } else if ui.button(RichText::new("Alles wissen").color(ERROR_RED)).clicked() {
                self.confirm_clear = true;
            }
        });
        if self.data.settings.source == Source::Live {
            ui.label(
                RichText::new("Live-modus staat aan: deze lijst wordt bij elke verversing overschreven met de echte NMBS-gegevens.")
                    .color(Color32::from_rgb(150, 110, 0)),
            );
        }
        ui.add_space(6.0);
        if self.data.trains.is_empty() {
            ui.label(RichText::new("Geen treinen. Voeg er een toe met “+ Trein”.").weak());
        }

        let open_by_default = self.data.trains.len() <= 12;
        let count = self.data.trains.len();
        let mut action = None;
        for (i, t) in self.data.trains.iter_mut().enumerate() {
            let title = format!(
                "{}  {}",
                t.time,
                if t.dest.is_empty() {
                    "(geen bestemming)"
                } else {
                    &t.dest
                }
            );
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                let id = ui.make_persistent_id(("train", t.id));
                egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, open_by_default)
                    .show_header(ui, |ui| {
                        ui.label(RichText::new(title).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button(RichText::new("×").color(ERROR_RED))
                                .on_hover_text("Verwijderen")
                                .clicked()
                            {
                                action = Some(TrainAction::Delete(i));
                            }
                            if ui.small_button("Kopie").on_hover_text("Dupliceren").clicked() {
                                action = Some(TrainAction::Duplicate(i));
                            }
                            if ui.add_enabled(i + 1 < count, egui::Button::new("▼").small()).clicked() {
                                action = Some(TrainAction::Down(i));
                            }
                            if ui.add_enabled(i > 0, egui::Button::new("▲").small()).clicked() {
                                action = Some(TrainAction::Up(i));
                            }
                        });
                    })
                    .body(|ui| changed |= train_editor(ui, t));
            });
        }
        if let Some(a) = action {
            let trains = &mut self.data.trains;
            match a {
                TrainAction::Up(i) => trains.swap(i, i - 1),
                TrainAction::Down(i) => trains.swap(i, i + 1),
                TrainAction::Duplicate(i) => {
                    let copy = trains[i].duplicate();
                    trains.insert(i + 1, copy);
                }
                TrainAction::Delete(i) => {
                    trains.remove(i);
                }
            }
            changed = true;
        }
        changed
    }

    // ---------- Live ----------

    fn live_tab(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        let s = &mut self.data.settings;
        labeled(ui, "Gegevensbron", |ui| {
            ComboBox::from_id_salt("source")
                .width(300.0)
                .selected_text(match s.source {
                    Source::Manual => "Handmatig / simulatie",
                    Source::Live => "Live NMBS-gegevens (iRail)",
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut s.source, Source::Manual, "Handmatig / simulatie")
                        .changed();
                    changed |= ui
                        .selectable_value(&mut s.source, Source::Live, "Live NMBS-gegevens (iRail)")
                        .changed();
                });
        });

        if self.data.settings.source == Source::Live {
            ui.add_space(4.0);
            let mut query = self.live.station_query.clone();
            let mut picked = None;
            labeled(ui, "Station (typ om te zoeken)", |ui| {
                let resp = ui.add(
                    TextEdit::singleline(&mut query)
                        .desired_width(300.0)
                        .hint_text("bv. Gent-Sint-Pieters"),
                );
                let matches = if query != self.data.settings.live_station_name {
                    self.station_matches(&query, 8)
                } else {
                    vec![]
                };
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    picked = matches.first().cloned();
                }
                for st in &matches {
                    if ui.selectable_label(false, &st.name).clicked() {
                        picked = Some(st.clone());
                    }
                }
                if self.live.stations.is_empty() {
                    ui.label(RichText::new("Stationslijst wordt geladen…").weak());
                } else if !query.trim().is_empty()
                    && matches.is_empty()
                    && query != self.data.settings.live_station_name
                {
                    ui.label(RichText::new("Geen station gevonden.").color(ERROR_RED));
                }
            });
            self.live.station_query = query;
            if let Some(st) = picked {
                self.select_station(&st);
                changed = true;
            }

            let s = &mut self.data.settings;
            labeled(ui, "Bord", |ui| {
                ComboBox::from_id_salt("arrdep")
                    .selected_text(if s.live_arrdep == ArrDep::Departure {
                        "Vertrek"
                    } else {
                        "Aankomst"
                    })
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(&mut s.live_arrdep, ArrDep::Departure, "Vertrek")
                            .changed();
                        changed |= ui
                            .selectable_value(&mut s.live_arrdep, ArrDep::Arrival, "Aankomst")
                            .changed();
                    });
            });
            labeled(ui, "Alleen deze sporen (leeg = alle, bv. \"3\" of \"11, 12\")", |ui| {
                changed |= text_field(ui, &mut s.live_platforms, 300.0, "alle sporen", true);
            });
            ui.horizontal(|ui| {
                labeled(ui, "Aantal via-stations", |ui| {
                    changed |= ui.add(DragValue::new(&mut s.live_via).range(0..=6)).changed();
                });
                labeled(ui, "Verversen elke (s)", |ui| {
                    changed |= ui
                        .add(DragValue::new(&mut s.live_refresh).range(30.0..=600.0).speed(1.0))
                        .changed();
                });
            });
            changed |= ui
                .checkbox(&mut s.live_hide_left, "Vertrokken treinen verbergen")
                .changed();
            changed |= ui
                .checkbox(&mut s.live_show_station, "Stationsnaam in kopbalk tonen")
                .changed();
            if ui.button("Nu verversen").clicked() {
                self.refresh_live_now();
            }
        }

        if !self.live.status.is_empty() {
            ui.add_space(6.0);
            let text = RichText::new(&self.live.status);
            ui.label(if self.live.status_is_error {
                text.color(ERROR_RED)
            } else {
                text
            });
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("Gegevens: de open ");
            ui.hyperlink_to("iRail API", "https://docs.irail.be");
            ui.label(
                " met de realtime-informatie van de NMBS (vertrektijden, vertragingen, sporen, \
                 spoorwijzigingen en afgeschafte treinen). Namen volgen de gekozen taal. \
                 Een internetverbinding is nodig.",
            );
        });
        changed
    }

    // ---------- General ----------

    fn general_tab(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        let s = &mut self.data.settings;
        labeled(ui, "Titel (leeg = automatisch volgens taal)", |ui| {
            changed |= text_field(ui, &mut s.title, 300.0, "Vertrek", true);
        });
        labeled(ui, "Taal", |ui| {
            ComboBox::from_id_salt("language")
                .width(300.0)
                .selected_text(s.language.label())
                .show_ui(ui, |ui| {
                    for l in Language::ALL {
                        changed |= ui.selectable_value(&mut s.language, l, l.label()).changed();
                    }
                });
        });
        ui.horizontal(|ui| {
            labeled(ui, "Wisselinterval taal (s)", |ui| {
                changed |= ui
                    .add(DragValue::new(&mut s.cycle_interval).range(2.0..=120.0).speed(0.2))
                    .changed();
            });
            labeled(ui, "Aantal rijen", |ui| {
                changed |= ui.add(DragValue::new(&mut s.rows_visible).range(3..=14)).changed();
            });
        });
        ui.add_space(4.0);
        for (value, label) in [
            (&mut s.show_logo, "NMBS-logo tonen"),
            (&mut s.show_seconds, "Seconden tonen in klok"),
            (&mut s.blink_colon, "Knipperende dubbele punt"),
            (&mut s.marquee, "Lange teksten laten scrollen"),
            (&mut s.show_via_word, "Woord \"via\" automatisch toevoegen"),
            (&mut s.alternate_rows, "Afwisselende rijkleuren"),
            (&mut s.show_empty_rows, "Lege rijen tonen"),
        ] {
            changed |= ui.checkbox(value, label).changed();
        }
        changed
    }

    // ---------- Clock ----------

    fn clock_tab(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        ui.strong("Klok");
        let old_mode = self.data.settings.clock_mode;
        let s = &mut self.data.settings;
        labeled(ui, "Klokmodus", |ui| {
            let label = |m: ClockMode| match m {
                ClockMode::Live => "Live (Belgische tijd)",
                ClockMode::Offset => "Live met verschuiving",
                ClockMode::Custom => "Eigen tijd (simulatie)",
            };
            ComboBox::from_id_salt("clockmode")
                .width(300.0)
                .selected_text(label(s.clock_mode))
                .show_ui(ui, |ui| {
                    for m in [ClockMode::Live, ClockMode::Offset, ClockMode::Custom] {
                        changed |= ui.selectable_value(&mut s.clock_mode, m, label(m)).changed();
                    }
                });
        });
        if s.clock_mode != old_mode {
            self.clock.restart(s);
        }

        match s.clock_mode {
            ClockMode::Offset => {
                labeled(ui, "Verschuiving (minuten, mag negatief)", |ui| {
                    changed |= ui
                        .add(DragValue::new(&mut s.clock_offset).range(-1440.0..=1440.0).speed(1.0))
                        .changed();
                });
            }
            ClockMode::Custom => {
                let mut restart = false;
                labeled(ui, "Starttijd (UU:MM of UU:MM:SS)", |ui| {
                    let valid = parse_hm(&s.custom_start).is_some();
                    if text_field(ui, &mut s.custom_start, 120.0, "17:20:00", valid) {
                        changed = true;
                        restart = valid;
                    }
                });
                let speeds = [
                    (0.0, "Bevroren (0×)"),
                    (1.0, "Normaal (1×)"),
                    (2.0, "2×"),
                    (5.0, "5×"),
                    (10.0, "10×"),
                    (30.0, "30×"),
                    (60.0, "60× (1 min/s)"),
                ];
                let current = speeds
                    .iter()
                    .find(|(v, _)| *v == s.clock_speed)
                    .map_or("Eigen", |(_, l)| l);
                let mut new_speed = s.clock_speed;
                labeled(ui, "Snelheid", |ui| {
                    ComboBox::from_id_salt("speed")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for (v, l) in speeds {
                                ui.selectable_value(&mut new_speed, v, l);
                            }
                        });
                });
                if new_speed != s.clock_speed {
                    self.clock.reanchor(s);
                    s.clock_speed = new_speed;
                    changed = true;
                }
                if ui.button("Klok herstarten vanaf starttijd").clicked() {
                    restart = true;
                }
                if restart {
                    self.clock.restart(s);
                }
            }
            ClockMode::Live => {}
        }

        ui.add_space(10.0);
        ui.strong("Automatisch");
        changed |= ui
            .checkbox(&mut s.auto_depart, "Treinen verdwijnen na vertrek")
            .changed();
        labeled(ui, "Blijft nog zichtbaar na vertrek (minuten)", |ui| {
            changed |= ui
                .add(DragValue::new(&mut s.depart_grace).range(0.0..=30.0).speed(0.1))
                .changed();
        });
        changed |= ui
            .add_enabled(
                s.source == Source::Manual,
                egui::Checkbox::new(&mut s.auto_fill, "Bord automatisch aanvullen met nieuwe treinen"),
            )
            .changed();
        changed |= ui.checkbox(&mut s.auto_sort, "Automatisch sorteren op tijd").changed();
        labeled(
            ui,
            "\"Komt aan\" automatisch tonen (minuten vóór vertrek, 0 = uit)",
            |ui| {
                changed |= ui
                    .add(DragValue::new(&mut s.auto_arriving).range(0.0..=30.0).speed(0.1))
                    .changed();
            },
        );
        changed
    }

    // ---------- Colours ----------

    fn colors_tab(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        egui::Grid::new("colors")
            .num_columns(2)
            .spacing([24.0, 8.0])
            .show(ui, |ui| {
                for (label, value) in self.data.settings.colors.fields_mut() {
                    ui.label(label);
                    let mut rgb = parse_hex(value);
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        *value = to_hex(rgb);
                        changed = true;
                    }
                    ui.end_row();
                }
            });
        ui.add_space(8.0);
        if ui.button("Standaardkleuren herstellen").clicked() {
            self.data.settings.colors = Default::default();
            changed = true;
        }
        changed
    }

    // ---------- Data ----------

    fn data_tab(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        ui.label("Alle instellingen worden automatisch bewaard in:");
        ui.label(
            RichText::new(self.config_path().display().to_string())
                .monospace()
                .small(),
        );
        ui.label(RichText::new("Het JSON-formaat is hetzelfde als de export van de webversie.").weak());
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Exporteren (JSON)").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name("nmbs-vertrekbord.json")
                    .save_file()
            {
                let result = serde_json::to_string_pretty(&self.data)
                    .map_err(|e| e.to_string())
                    .and_then(|json| std::fs::write(&path, json).map_err(|e| e.to_string()));
                self.message = Some(match result {
                    Ok(()) => format!("Geëxporteerd naar {}", path.display()),
                    Err(e) => format!("Exporteren mislukt: {e}"),
                });
            }
            if ui.button("Importeren").clicked()
                && let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file()
            {
                let result = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|text| serde_json::from_str::<SaveData>(&text).map_err(|e| e.to_string()));
                match result {
                    Ok(data) => {
                        self.replace_data(data);
                        self.message = Some(format!("Geïmporteerd uit {}", path.display()));
                    }
                    Err(e) => self.message = Some(format!("Kon bestand niet lezen: {e}")),
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if self.confirm_reset {
                if ui
                    .button(RichText::new("Zeker? Alles terugzetten").color(ERROR_RED))
                    .clicked()
                {
                    self.replace_data(SaveData::defaults());
                    self.confirm_reset = false;
                    changed = true;
                }
                if ui.button("Annuleren").clicked() {
                    self.confirm_reset = false;
                }
            } else if ui
                .button(RichText::new("Alles terugzetten naar standaard").color(ERROR_RED))
                .clicked()
            {
                self.confirm_reset = true;
            }
        });
        if let Some(msg) = &self.message {
            ui.label(msg);
        }
        ui.add_space(10.0);
        ui.strong("Sneltoetsen");
        ui.label("P  paneel tonen/verbergen\nF of F11  volledig scherm (of dubbelklik op het bord)\nR  willekeurige trein toevoegen\nEsc  paneel sluiten");
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Start met --display om het paneel verborgen te houden, of --fullscreen voor volledig scherm.",
            )
            .weak(),
        );
        changed
    }
}

/// Editor for one train; returns true if something changed.
fn train_editor(ui: &mut Ui, t: &mut Train) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        labeled(ui, "Tijd", |ui| {
            let valid = parse_hm(&t.time).is_some();
            changed |= text_field(ui, &mut t.time, 60.0, "UU:MM", valid);
        });
        labeled(ui, "Spoor", |ui| {
            changed |= text_field(ui, &mut t.platform, 50.0, "", true)
        });
        labeled(ui, "Type", |ui| {
            changed |= text_field(ui, &mut t.kind, 60.0, "IC", true)
        });
        labeled(ui, "Icoon", |ui| {
            ComboBox::from_id_salt(("icon", t.id))
                .width(110.0)
                .selected_text(if t.icon == Icon::International {
                    "Internationaal"
                } else {
                    "Geen (tekst)"
                })
                .show_ui(ui, |ui| {
                    changed |= ui.selectable_value(&mut t.icon, Icon::None, "Geen (tekst)").changed();
                    changed |= ui
                        .selectable_value(&mut t.icon, Icon::International, "Internationaal")
                        .changed();
                });
        });
    });
    let wide = (ui.available_width() - 8.0).max(200.0);
    labeled(ui, "Bestemming", |ui| {
        changed |= text_field(ui, &mut t.dest, wide, "", true)
    });
    labeled(ui, "Via", |ui| {
        changed |= text_field(ui, &mut t.via, wide, "Halle, Bergen", true)
    });
    labeled(ui, "Opmerking (vervangt via-regel)", |ui| {
        changed |= text_field(ui, &mut t.remark, wide, "Rijdt niet verder dan Brugge", true);
    });
    ui.horizontal(|ui| {
        labeled(ui, "Vertraging (min)", |ui| {
            changed |= ui.add(DragValue::new(&mut t.delay).range(0..=999)).changed()
        });
        labeled(ui, "Verwachte tijd", |ui| {
            let valid = time_valid(&t.expected);
            changed |= text_field(ui, &mut t.expected, 60.0, "auto", valid);
        });
        labeled(ui, "Status", |ui| {
            ComboBox::from_id_salt(("status", t.id))
                .width(110.0)
                .selected_text(t.status.label())
                .show_ui(ui, |ui| {
                    for st in Status::ALL {
                        changed |= ui.selectable_value(&mut t.status, st, st.label()).changed();
                    }
                });
        });
    });
    if t.status == Status::Custom {
        ui.horizontal(|ui| {
            labeled(ui, "Eigen status", |ui| {
                changed |= text_field(ui, &mut t.custom_status, 160.0, "", true)
            });
            labeled(ui, "Statuskleur", |ui| {
                let mut rgb = parse_hex(&t.custom_color);
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    t.custom_color = to_hex(rgb);
                    changed = true;
                }
            });
        });
    }
    changed |= ui
        .checkbox(&mut t.platform_changed, "Spoorwijziging markeren")
        .changed();
    changed
}
