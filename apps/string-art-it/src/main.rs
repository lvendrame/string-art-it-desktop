//! StringArtIt desktop — native egui port of the web editor.

use string_art_it::{app, icons};

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("StringArtIt")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 640.0])
            .with_icon(icons::app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "StringArtIt",
        options,
        Box::new(|cc| Ok(Box::new(app::StringArtApp::new(cc)))),
    )
}
