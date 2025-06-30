use eframe;

mod app;
mod download;
mod localizations;
mod models;
mod ui;

use app::YtdlApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_min_inner_size([400.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "YouTube Downloader",
        options,
        Box::new(|_cc| Box::<YtdlApp>::default()),
    )
}