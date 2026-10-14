//! Paints the departure board. All geometry is in "board units" of a
//! 1280×720 screen and scaled to the available area, like the web version.

use crate::model::{Icon, LangTexts, Settings, Status, Train, fmt_hm, minutes_until, parse_hex};
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Align, Color32, FontFamily, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, vec2};

pub const BOARD_W: f32 = 1280.0;
pub const BOARD_H: f32 = 720.0;
const HEADER_H: f32 = 76.0;
const BASE_ROW_H: f32 = (BOARD_H - HEADER_H) / 8.0;
const BACKDROP: Color32 = Color32::from_rgb(5, 16, 31);

pub const REGULAR: &str = "regular";
pub const SEMIBOLD: &str = "semibold";
pub const BOLD: &str = "bold";

fn family(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

fn color(hex: &str) -> Color32 {
    let [r, g, b] = parse_hex(hex);
    Color32::from_rgb(r, g, b)
}

/// Everything the board needs for one frame.
pub struct BoardView<'a> {
    pub settings: &'a Settings,
    pub trains: &'a [Train],
    pub texts: &'a LangTexts,
    /// Current board time in seconds of the day.
    pub now_sec: f64,
    /// Monotonic time in seconds, for animations.
    pub anim_time: f64,
}

enum Badge {
    Delay { delay: i64, new_time: String },
    Status { text: String, bg: Color32 },
}

/// Maps board units to screen points.
struct Canvas<'p> {
    painter: &'p Painter,
    origin: Pos2,
    scale: f32,
}

impl Canvas<'_> {
    fn pos(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.scale
    }

    fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.pos(x, y), vec2(w, h) * self.scale)
    }

    fn fill(&self, x: f32, y: f32, w: f32, h: f32, c: Color32) {
        self.painter.rect_filled(self.rect(x, y, w, h), 0.0, c);
    }

    fn font(&self, size: f32, fam: &str) -> FontId {
        FontId::new(size * self.scale, family(fam))
    }

    /// Lays out one line of text; returns the galley and its size in board units.
    fn layout(
        &self,
        text: &str,
        size: f32,
        fam: &str,
        c: Color32,
        max_w: Option<f32>,
    ) -> (std::sync::Arc<egui::Galley>, Vec2) {
        let mut job = LayoutJob::single_section(text.to_owned(), TextFormat::simple(self.font(size, fam), c));
        if let Some(w) = max_w {
            job.wrap = TextWrapping::truncate_at_width(w * self.scale);
        }
        let galley = self.painter.layout_job(job);
        let size = galley.size() / self.scale;
        (galley, size)
    }

    /// Draws text vertically centred in the line box `[top, top + line_h]`.
    #[allow(clippy::too_many_arguments)]
    fn text(
        &self,
        painter: &Painter,
        x: f32,
        top: f32,
        line_h: f32,
        text: &str,
        size: f32,
        fam: &str,
        c: Color32,
        max_w: Option<f32>,
    ) -> f32 {
        let (galley, sz) = self.layout(text, size, fam, c, max_w);
        painter.galley(self.pos(x, top + (line_h - sz.y) / 2.0), galley, c);
        sz.x
    }
}

/// Paints the board into `area`. Returns true while something animates.
pub fn paint(painter: &Painter, area: Rect, view: &BoardView) -> bool {
    painter.rect_filled(area, 0.0, BACKDROP);
    let scale = (area.width() / BOARD_W).min(area.height() / BOARD_H);
    let origin = area.center() - vec2(BOARD_W, BOARD_H) * scale / 2.0;
    let board_rect = Rect::from_min_size(origin, vec2(BOARD_W, BOARD_H) * scale);
    let painter = painter.with_clip_rect(board_rect);
    let cv = Canvas {
        painter: &painter,
        origin,
        scale,
    };
    let s = view.settings;
    let c = &s.colors;

    cv.fill(0.0, 0.0, BOARD_W, BOARD_H, color(&c.row_a));
    let mut animating = paint_header(&cv, view);

    let n = s.rows();
    let row_h = (BOARD_H - HEADER_H) / n as f32;
    let k = (row_h / BASE_ROW_H).clamp(0.5, 1.25);
    for i in 0..n {
        let train = view.trains.get(i);
        if train.is_none() && !s.show_empty_rows {
            continue;
        }
        let y0 = HEADER_H + i as f32 * row_h;
        let bg = if s.alternate_rows && i % 2 == 1 {
            &c.row_b
        } else {
            &c.row_a
        };
        cv.fill(0.0, y0, BOARD_W, row_h, color(bg));
        cv.fill(0.0, y0, BOARD_W, 2.0 * k, color(&c.separator));
        if let Some(t) = train {
            animating |= paint_row(&cv, view, t, y0 + 2.0 * k, row_h - 2.0 * k, k);
        }
    }
    animating
}

fn paint_header(cv: &Canvas, view: &BoardView) -> bool {
    let s = view.settings;
    let white = color(&s.colors.text);
    cv.fill(0.0, 0.0, BOARD_W, HEADER_H, color(&s.colors.header));

    // Clock: HH:MM, optionally :SS and a blinking colon.
    let secs = view.now_sec as i64;
    let colon_off = s.blink_colon && (view.anim_time * 2.0) as i64 % 2 == 1;
    let colon_color = if colon_off { Color32::TRANSPARENT } else { white };
    let big = TextFormat {
        font_id: cv.font(32.0, BOLD),
        color: white,
        valign: Align::BOTTOM,
        ..Default::default()
    };
    let small = TextFormat {
        font_id: cv.font(24.0, BOLD),
        color: white,
        valign: Align::BOTTOM,
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    job.append(&format!("{:02}", secs / 3600 % 24), 0.0, big.clone());
    job.append(
        ":",
        0.0,
        TextFormat {
            color: colon_color,
            ..big.clone()
        },
    );
    job.append(&format!("{:02}", secs / 60 % 60), 0.0, big);
    if s.show_seconds {
        job.append(
            ":",
            0.0,
            TextFormat {
                color: colon_color,
                ..small.clone()
            },
        );
        job.append(&format!("{:02}", secs % 60), 0.0, small);
    }
    let galley = cv.painter.layout_job(job);
    let h = galley.size().y / cv.scale;
    cv.painter.galley(cv.pos(22.0, (HEADER_H - h) / 2.0), galley, white);

    // Title
    let is_arrival = s.source == crate::model::Source::Live && s.live_arrdep == crate::model::ArrDep::Arrival;
    let mut title = if s.title.trim().is_empty() {
        if is_arrival {
            view.texts.arr_title
        } else {
            view.texts.title
        }
        .to_owned()
    } else {
        s.title.clone()
    };
    if s.source == crate::model::Source::Live && s.live_show_station && !s.live_station_name.is_empty() {
        title.push_str(" – ");
        title.push_str(&s.live_station_name);
    }
    cv.text(
        cv.painter,
        290.0,
        0.0,
        HEADER_H,
        &title,
        32.0,
        BOLD,
        white,
        Some(BOARD_W - 290.0 - 110.0),
    );

    // NMBS logo: a "B" in an oval.
    if s.show_logo {
        let center = cv.pos(BOARD_W - 22.0 - 32.0, HEADER_H / 2.0);
        cv.painter.add(Shape::ellipse_stroke(
            center,
            vec2(29.0, 19.0) * cv.scale,
            Stroke::new(3.5 * cv.scale, white),
        ));
        let (galley, sz) = cv.layout("B", 31.0, BOLD, white, None);
        cv.painter
            .galley(center - sz * cv.scale / 2.0 + vec2(0.5, 0.5) * cv.scale, galley, white);
    }
    s.blink_colon
}

fn badge_for(t: &Train, view: &BoardView) -> Option<Badge> {
    let c = &view.settings.colors;
    let status = |text: &str, bg: &str| {
        Some(Badge::Status {
            text: text.to_owned(),
            bg: color(bg),
        })
    };
    match t.status {
        Status::Custom if !t.custom_status.is_empty() => return status(&t.custom_status, &t.custom_color),
        Status::Limited => return status(view.texts.limited, &c.alert),
        Status::Cancelled => return status(view.texts.cancelled, &c.alert),
        Status::Arriving => return status(view.texts.arriving, &c.info),
        _ => {}
    }
    if t.delay > 0 || !t.expected.is_empty() {
        let exp = t.expected_min();
        let delay = if t.delay > 0 {
            t.delay
        } else {
            minutes_until(exp, t.scheduled_min()).round() as i64
        };
        if delay > 0 {
            return Some(Badge::Delay {
                delay,
                new_time: fmt_hm(exp),
            });
        }
    }
    let auto = view.settings.auto_arriving;
    if auto > 0.0 {
        let u = minutes_until(t.expected_min(), view.now_sec / 60.0);
        if (0.0..=auto).contains(&u) {
            return status(view.texts.arriving, &c.info);
        }
    }
    None
}

/// Paints one train row. `top`/`h` describe the row content below the
/// separator; `k` is the row zoom factor. Returns true if it animates.
fn paint_row(cv: &Canvas, view: &BoardView, t: &Train, top: f32, h: f32, k: f32) -> bool {
    let s = view.settings;
    let c = &s.colors;
    let white = color(&c.text);
    let dim = if t.status == Status::Cancelled { 0.75 } else { 1.0 };
    let mut animating = false;

    // Time + badge
    cv.text(
        cv.painter,
        22.0 * k,
        top + 6.0 * k,
        30.0 * k,
        &t.time,
        30.0 * k,
        BOLD,
        white,
        None,
    );
    if let Some(badge) = badge_for(t, view) {
        let by = top + h - 8.0 * k - 30.0 * k;
        let bh = 30.0 * k;
        match badge {
            Badge::Delay { delay, new_time } => {
                cv.fill(0.0, by, 128.0 * k, bh, color(&c.alert));
                cv.text(
                    cv.painter,
                    14.0 * k,
                    by,
                    bh,
                    &format!("+{delay}'"),
                    25.0 * k,
                    SEMIBOLD,
                    white,
                    Some(110.0 * k),
                );
                cv.fill(128.0 * k, by, 112.0 * k, bh, color(&c.new_time_bg));
                let (galley, sz) = cv.layout(&new_time, 25.0 * k, SEMIBOLD, color(&c.new_time_text), None);
                cv.painter.galley(
                    cv.pos(128.0 * k + (112.0 * k - sz.x) / 2.0, by + (bh - sz.y) / 2.0),
                    galley,
                    white,
                );
            }
            Badge::Status { text, bg } => {
                cv.fill(0.0, by, 240.0 * k, bh, bg);
                cv.text(
                    cv.painter,
                    14.0 * k,
                    by,
                    bh,
                    &text,
                    25.0 * k,
                    SEMIBOLD,
                    white,
                    Some(222.0 * k),
                );
            }
        }
    }

    // Destination + via / remark
    let main_x = 290.0 * k;
    let main_w = BOARD_W - 270.0 * k - main_x;
    let dest_line = 31.0 * 1.1 * k;
    cv.text(
        cv.painter,
        main_x,
        top + 4.0 * k,
        dest_line,
        &t.dest,
        31.0 * k,
        BOLD,
        color(&c.dest).gamma_multiply(dim),
        Some(main_w),
    );

    let second = if !t.remark.is_empty() {
        t.remark.clone()
    } else if !t.via.is_empty() {
        if s.show_via_word {
            format!("{} {}", view.texts.via, t.via)
        } else {
            t.via.clone()
        }
    } else {
        String::new()
    };
    if !second.is_empty() {
        let via_top = top + 4.0 * k + dest_line;
        let via_line = 26.0 * 1.25 * k;
        let (galley, sz) = cv.layout(&second, 26.0 * k, REGULAR, white.gamma_multiply(dim), None);
        let overflow = sz.x - main_w;
        let mut offset = 0.0;
        if s.marquee && overflow > 4.0 {
            let dist = overflow + 10.0;
            let dur = (overflow / 40.0 + 5.0).max(6.0) as f64;
            let phase = (view.anim_time % dur / dur) as f32;
            offset = ((phase - 0.15) / 0.7).clamp(0.0, 1.0) * dist;
            animating = true;
        }
        let clip = cv.rect(main_x, via_top, main_w, via_line);
        cv.painter.with_clip_rect(clip).galley(
            cv.pos(main_x - offset, via_top + (via_line - sz.y) / 2.0),
            galley,
            white,
        );
    }

    // Platform
    if !t.platform.is_empty() {
        let fg = if t.platform_changed {
            color(&c.new_time_text)
        } else {
            white
        };
        let (galley, sz) = cv.layout(&t.platform, 29.0 * k, BOLD, fg, None);
        let w = (sz.x + 18.0 * k).max(58.0 * k);
        let bx = BOARD_W - 200.0 * k - w / 2.0;
        let by = top + (h - 58.0 * k) / 2.0;
        let rect = cv.rect(bx, by, w, 58.0 * k);
        let radius = 7.0 * k * cv.scale;
        if t.platform_changed {
            cv.painter.rect_filled(rect, radius, color(&c.platform_change));
        } else {
            cv.painter
                .rect_stroke(rect, radius, Stroke::new(3.0 * k * cv.scale, white), StrokeKind::Inside);
        }
        cv.painter
            .galley(cv.pos(bx + (w - sz.x) / 2.0, by + (58.0 * k - sz.y) / 2.0), galley, fg);
    }

    // Train type or international icon
    let type_x = BOARD_W - 130.0 * k + 30.0 * k;
    if t.icon == Icon::International {
        let size = 50.0 * k * cv.scale;
        let center = cv.pos(type_x - 6.0 * k + 25.0 * k, top + h / 2.0);
        paint_international(cv.painter, center, size, white);
    } else if !t.kind.is_empty() {
        cv.text(
            cv.painter,
            type_x,
            top,
            h,
            &t.kind,
            30.0 * k,
            BOLD,
            white,
            Some(120.0 * k),
        );
    }
    animating
}

/// Circle with a six-pointed asterisk, as on the NMBS boards.
fn paint_international(painter: &Painter, center: Pos2, size: f32, c: Color32) {
    let u = size / 50.0;
    let stroke = Stroke::new(3.0 * u, c);
    painter.circle_stroke(center, 21.0 * u, stroke);
    for angle in [90.0_f32, 30.0, -30.0] {
        let d = Vec2::angled(angle.to_radians()) * 14.0 * u;
        painter.line_segment([center - d, center + d], stroke);
    }
    painter.circle_filled(center, 4.0 * u, c);
}
