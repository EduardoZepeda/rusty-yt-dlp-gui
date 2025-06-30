use eframe::egui::{self, Color32, Stroke};
use rfd::FileDialog;
use std::path::Path;

use crate::localizations::Localizations;
use crate::models::{AppState, DownloadFormat};

use crate::theme::*;

pub fn render_url_input(
    ui: &mut egui::Ui,
    state: &mut AppState,
    localizer: &Localizations,
) -> egui::Response {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(
                localizer
                    .lookup_single_language("url-label", None)
                    .unwrap_or_else(|| "Video URL".to_string()),
            )
            .color(MAIN_TEXT)
            .size(14.0),
        );

        egui::Frame::none()
            .fill(CARD_COLOR)
            .rounding(ROUNDING_FRAME)
            .stroke(Stroke::new(1.0, BORDER_COLOR))
            .show(ui, |ui| {
                ui.add_sized(
                    [ui.available_width(), 48.0],
                    egui::TextEdit::singleline(&mut state.url)
                        .hint_text(
                            localizer
                                .lookup_single_language("url-placeholder", None)
                                .unwrap_or_else(|| "Enter video URL".to_string()),
                        )
                        .text_color(SECONDARY_TEXT)
                        .frame(false)
                        .font(egui::TextStyle::Body)
                        .font(egui::FontId::proportional(15.0)),
                )
            })
            .inner
    })
    .inner
}

pub fn render_format_selector(ui: &mut egui::Ui, state: &mut AppState, localizer: &Localizations) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(
                localizer
                    .lookup_single_language("download-format", None)
                    .unwrap_or_else(|| "Download as".to_string()),
            )
            .color(MAIN_TEXT)
            .size(14.0),
        );

        ui.horizontal(|ui| {
            let mp4_label = localizer
                .lookup_single_language("format-mp4", None)
                .unwrap_or_else(|| "MP4 (Video)".to_string());
            let mp3_label = localizer
                .lookup_single_language("format-mp3", None)
                .unwrap_or_else(|| "MP3 (Audio only)".to_string());

            let is_mp4 = state.format == DownloadFormat::MP4;

            // Format selection using radio buttons
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 20.0; // Add some space between radio buttons
                
                // MP4 Radio
                ui.radio_value(
                    &mut state.format,
                    DownloadFormat::MP4,
                    egui::RichText::new(mp4_label)
                        .color(MAIN_TEXT)
                        .size(14.0)
                );
                
                // MP3 Radio
                ui.radio_value(
                    &mut state.format,
                    DownloadFormat::MP3,
                    egui::RichText::new(mp3_label)
                        .color(MAIN_TEXT)
                        .size(14.0)
                );
            });
        });
    });
}

pub fn render_download_dir_selector(
    ui: &mut egui::Ui,
    state: &mut AppState,
    localizer: &Localizations,
) -> bool {
    let mut changed = false;

    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(
                localizer
                    .lookup_single_language("download-to", None)
                    .unwrap_or_else(|| "Download to".to_string()),
            )
            .color(MAIN_TEXT)
            .size(14.0),
        );

        ui.horizontal(|ui| {
            // Directory path display
            egui::Frame::none()
                .fill(CARD_COLOR)
                .rounding(ROUNDING_FRAME)
                .stroke(Stroke::new(1.0, BORDER_COLOR))
                .show(ui, |ui| {
                    ui.set_min_height(36.0);
                    let response = ui.add_sized(
                        [ui.available_width() - 120.0, 36.0],
                        egui::TextEdit::singleline(&mut state.download_dir)
                            .hint_text(
                                localizer
                                    .lookup_single_language("select-directory", None)
                                    .unwrap_or_else(|| "Select download directory".to_string()),
                            )
                            .text_color(SECONDARY_TEXT)
                            .font(egui::FontId::proportional(14.0))
                            .frame(false)
                            .margin(egui::vec2(12.0, 0.0)),
                    );
                    changed = response.changed();
                });

            // Browse button
            let button = egui::Button::new(
                egui::RichText::new(
                    localizer
                        .lookup_single_language("browse-button", None)
                        .unwrap_or_else(|| "Browse...".to_string()),
                )
                .color(Color32::WHITE)
                .size(14.0),
            )
            .min_size(egui::vec2(100.0, 36.0))
            .fill(PRIMARY_COLOR)
            .rounding(ROUNDING_BUTTON);

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
    egui::Frame::none()
        .rounding(ROUNDING_FRAME)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.add_space(12.0);

                let status_text = if let Some(error) = &state.last_error {
                    egui::RichText::new(format!("Error: {}", error))
                        .color(TEXT_ERROR)
                        .size(14.0)
                } else {
                    egui::RichText::new(&state.status)
                        .color(TEXT_SUCCESS)
                        .size(14.0)
                };

                ui.label(status_text);
                ui.add_space(8.0);

                if state.is_downloading {
                    let progress_text = if !state.download_speed.is_empty() {
                        format!(
                            "{} • {} • ETA: {}",
                            state.status, state.download_speed, state.eta
                        )
                    } else {
                        localizer
                            .lookup_single_language("status-downloading", None)
                            .unwrap_or_else(|| "Starting download...".to_string())
                    };

                    // Progress bar with custom styling
                    let progress_bar = egui::ProgressBar::new(state.progress / 100.0)
                        .show_percentage()
                        .text(progress_text)
                        .fill(PRIMARY_COLOR);

                    ui.add(progress_bar);

                    // Progress text below the bar
                    ui.label(
                        egui::RichText::new(format!("{:.1}% complete", state.progress))
                            .color(SECONDARY_TEXT)
                            .size(12.0),
                    );
                }

                if let Some(path) = &state.output_path {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Saved to: ")
                                .color(SECONDARY_TEXT)
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(path.display().to_string())
                                .color(PRIMARY_COLOR)
                                .size(13.0),
                        );
                    });
                }

                ui.add_space(16.0);
            });
        });
}
