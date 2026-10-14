// Hide the console window on Windows in release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod board;
mod clock;
mod live;
mod model;
mod panel;

use std::sync::Arc;

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, bytes) in [
        (
            board::REGULAR,
            &include_bytes!("../assets/fonts/SourceSans3-Regular.ttf")[..],
        ),
        (
            board::SEMIBOLD,
            &include_bytes!("../assets/fonts/SourceSans3-SemiBold.ttf")[..],
        ),
        (board::BOLD, &include_bytes!("../assets/fonts/SourceSans3-Bold.ttf")[..]),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(egui::FontData::from_static(bytes)));
        // Fall back to egui's default fonts for symbols Source Sans lacks.
        let mut family = vec![name.to_owned()];
        family.extend(fonts.families[&egui::FontFamily::Proportional].iter().cloned());
        fonts.families.insert(egui::FontFamily::Name(name.into()), family);
    }
    // Use Source Sans for the settings panel too.
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .expect("proportional family")
        .insert(0, board::REGULAR.to_owned());
    ctx.set_fonts(fonts);
}

/// A simple window icon: the NMBS-blue square with a white oval.
fn icon() -> egui::IconData {
    const N: u32 = 64;
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let dx = (x as f32 + 0.5 - 32.0) / 26.0;
            let dy = (y as f32 + 0.5 - 32.0) / 17.0;
            let d = (dx * dx + dy * dy).sqrt();
            let ring = (d - 1.0).abs() < 0.12;
            let b_bar = (x as f32 - 27.0).abs() < 2.5 && (y as f32 - 32.0).abs() < 11.0;
            let b_bowl = {
                let bx = (x as f32 - 31.0) / 7.0;
                let by = ((y as f32 - 32.0).abs() - 5.5) / 5.5;
                let r = (bx * bx + by * by).sqrt();
                x as f32 >= 27.0 && (r - 1.0).abs() < 0.35
            };
            let px = if ring || b_bar || b_bowl {
                [255, 255, 255, 255]
            } else {
                [21, 96, 189, 255]
            };
            rgba.extend_from_slice(&px);
        }
    }
    egui::IconData {
        rgba,
        width: N,
        height: N,
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "NMBS Vertrekbord {}\n\nGebruik: nmbs-vertrekbord [--display] [--fullscreen]\n\n  \
             --display     start zonder instellingenpaneel\n  \
             --fullscreen  start in volledig scherm\n\nInstellingen: {}",
            env!("CARGO_PKG_VERSION"),
            app::config_dir().join("config.json").display()
        );
        return Ok(());
    }
    let display = args.iter().any(|a| a == "--display");
    let fullscreen = args.iter().any(|a| a == "--fullscreen");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NMBS Vertrekbord")
            .with_inner_size([if display { 1280.0 } else { 1720.0 }, 760.0])
            .with_min_inner_size([480.0, 300.0])
            .with_fullscreen(fullscreen)
            .with_icon(icon()),
        ..Default::default()
    };
    eframe::run_native(
        "NMBS Vertrekbord",
        options,
        Box::new(move |cc| {
            setup_fonts(&cc.egui_ctx);
            cc.egui_ctx.set_theme(egui::Theme::Light);
            Ok(Box::new(app::BoardApp::new(&cc.egui_ctx, !display)))
        }),
    )
}
