use eframe::egui;
use rfd::FileDialog;
use std::path::Path;

use crate::models::{AppState, DownloadFormat};
use crate::localizations::Localizations;

pub fn render_url_input(ui: &mut egui::Ui, state: &mut AppState, localizer: &Localizations) -> egui::Response {
    ui.label(
        localizer
            .lookup_single_language("url-label", None)
            .unwrap_or_else(|| "Video URL:".to_string()),
    );

    let response = egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(250, 250, 250))
        .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY))
        .rounding(4.0)
        .show(ui, |ui| {
            ui.add_sized(
                [ui.available_width(), 40.0],
                egui::TextEdit::singleline(&mut state.url)
                    .hint_text(
                        localizer
                            .lookup_single_language("url-placeholder", None)
                            .unwrap_or_else(|| "Enter video URL".to_string()),
                    )
                    .font(egui::TextStyle::Body)
                    .font(egui::FontId::proportional(16.0)),
            )
        })
        .inner;

    response
}

pub fn render_format_selector(ui: &mut egui::Ui, state: &mut AppState, localizer: &Localizations) {
    ui.horizontal(|ui| {
        ui.label(
            localizer
                .lookup_single_language("download-format", None)
                .unwrap_or_else(|| "Download as:".to_string()),
        );

        let mp4_label = localizer
            .lookup_single_language("format-mp4", None)
            .unwrap_or_else(|| "MP4 (Video)".to_string());
        let mp3_label = localizer
            .lookup_single_language("format-mp3", None)
            .unwrap_or_else(|| "MP3 (Audio only)".to_string());

        ui.radio_value(&mut state.format, DownloadFormat::MP4, mp4_label);
        ui.radio_value(&mut state.format, DownloadFormat::MP3, mp3_label);
    });
}

pub fn render_download_dir_selector(ui: &mut egui::Ui, state: &mut AppState, localizer: &Localizations) -> bool {
    let mut changed = false;
    
    ui.vertical(|ui| {
        ui.label(
            localizer
                .lookup_single_language("download-to", None)
                .unwrap_or_else(|| "Download to:".to_string()),
        );

        ui.horizontal(|ui| {
            egui::Frame::none()
                .fill(ui.visuals().extreme_bg_color)
                .rounding(4.0)
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .show(ui, |ui| {
                    ui.set_min_height(36.0);
                    let response = ui.add_sized(
                        [ui.available_width() - 100.0, 36.0],
                        egui::TextEdit::singleline(&mut state.download_dir)
                            .hint_text("Select download directory")
                            .frame(false)
                            .margin(egui::vec2(8.0, 8.0)),
                    );
                    changed = response.changed();
                });

            let button = egui::Button::new(
                egui::RichText::new(
                    localizer
                        .lookup_single_language("browse-button", None)
                        .unwrap_or_else(|| "Browse...".to_string()),
                )
                .size(14.0),
            )
            .min_size(egui::vec2(100.0, 36.0))
            .frame(true)
            .fill(ui.visuals().widgets.inactive.bg_fill)
            .rounding(4.0);

            if ui.add(button).clicked() {
                if let Some(path) = FileDialog::new()
                    .set_directory(
                        Path::new(&state.download_dir)
                            .parent()
                            .unwrap_or_else(|| Path::new(".")),
                    )
                    .pick_folder()
                {
                    state.download_dir = path.to_string_lossy().to_string();
                    changed = true;
                }
            }
        });
    });
    
    changed
}

pub fn render_status(ui: &mut egui::Ui, state: &AppState, localizer: &Localizations) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(248, 248, 248))
        .rounding(8.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.add_space(10.0);

                let status_text = if let Some(error) = &state.last_error {
                    egui::RichText::new(format!("Error: {}", error))
                        .color(egui::Color32::RED)
                } else {
                    egui::RichText::new(&state.status).color(egui::Color32::DARK_GRAY)
                };
                ui.label(status_text);

                if state.is_downloading {
                    ui.add_space(10.0);
                    let progress = state.progress / 100.0;
                    let progress_bar = egui::ProgressBar::new(progress)
                        .show_percentage()
                        .text(if progress > 0.0 {
                            format!("{} - {}", state.download_speed, state.eta)
                        } else {
                            localizer
                                .lookup_single_language("status-downloading", None)
                                .unwrap_or_else(|| "Starting download...".to_string())
                        });
                    ui.add(progress_bar);
                }

                if let Some(path) = &state.output_path {
                    ui.add_space(10.0);
                    ui.label(format!("Saved to: {}", path.display()));
                }

                ui.add_space(10.0);
            });
        });
}
