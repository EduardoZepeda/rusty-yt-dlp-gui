use eframe::egui::{self, Color32, Stroke};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::download::{start_download, update_ytdlp};
use crate::localizations::Localizations;
use crate::models::AppState;
use crate::theme::*;
use crate::ui;

pub struct YtdlApp {
    pub state: AppState,
    localizer: Localizations,
    status_sender: Sender<(bool, String)>,
    status_receiver: Option<Receiver<(bool, String)>>,
}

impl Default for YtdlApp {
    fn default() -> Self {
        Self::new()
    }
}

impl YtdlApp {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let localizer = Localizations::new();

        let mut state = AppState::default();
        state.status = localizer
            .lookup_single_language("status-ready", None)
            .unwrap_or_else(|| "Ready".to_string());

        Self {
            state,
            localizer,
            status_sender: tx,
            status_receiver: Some(rx),
        }
    }

    pub fn start_download(&mut self, ctx: &egui::Context) {
        if self.state.is_downloading {
            return;
        }

        if self.state.url.trim().is_empty() {
            self.state.error = Some(
                self.localizer
                    .lookup_single_language("enter-url", None)
                    .unwrap_or_else(|| "Please enter a URL".to_string()),
            );
            self.state.last_error = Some(
                self.localizer
                    .lookup_single_language("no-url", None)
                    .unwrap_or_else(|| "No URL provided".to_string()),
            );
            return;
        }

        self.state.is_downloading = true;
        self.state.progress = 0.0;
        self.state.error = None;
        self.state.last_error = None;
        self.state.download_speed = String::new();
        self.state.eta = String::new();
        self.state.status = self
            .localizer
            .lookup_single_language("status-downloading", None)
            .unwrap_or_else(|| "Downloading...".to_string());

        let format = self.state.format;
        let url = self.state.url.clone();
        let tx = self.status_sender.clone();

        start_download(url, format, tx);
        ctx.request_repaint();
    }

    pub fn update_ytdlp(&mut self, ctx: &egui::Context) {
        if self.state.is_downloading {
            return;
        }

        self.state.is_downloading = true;
        self.state.progress = 0.0;
        self.state.error = None;
        self.state.last_error = None;
        self.state.status = self
            .localizer
            .lookup_single_language("status-updating", None)
            .unwrap_or_else(|| "Updating yt-dlp...".to_string());

        let tx = self.status_sender.clone();
        update_ytdlp(tx);
        ctx.request_repaint();
    }

    pub fn update_ui(&mut self, ctx: &egui::Context) {
        self.process_status_updates(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(
                self.localizer
                    .lookup_single_language("app-title", None)
                    .unwrap_or_else(|| "YouTube Downloader".to_string()),
            );

            ui.add_space(20.0);

            let url_response = ui::render_url_input(ui, &mut self.state, &self.localizer);
            if url_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.start_download(ctx);
            }

            ui.add_space(10.0);
            ui::render_format_selector(ui, &mut self.state, &self.localizer);
            ui.add_space(20.0);

            ui::render_download_dir_selector(ui, &mut self.state, &self.localizer);
            ui.add_space(20.0);

            ui::render_status(ui, &self.state, &self.localizer);
            ui.add_space(ui.available_height() - 100.0);

            self.render_buttons(ui, ctx);
        });
    }

    fn process_status_updates(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &mut self.status_receiver {
            while let Ok((is_error, message)) = receiver.try_recv() {
                if message.contains('%') {
                    if let Some(percent_str) = message.split('%').next() {
                        if let Ok(percent) = percent_str.trim().parse::<f32>() {
                            self.state.progress = percent;
                            let parts: Vec<&str> = message.split_whitespace().collect();
                            if parts.len() >= 4 && parts[2] == "ETA:" {
                                self.state.download_speed = parts[1].to_string();
                                self.state.eta = parts[3].to_string();
                            }
                            self.state.status = message.clone();
                            ctx.request_repaint();
                            continue;
                        }
                    }
                }

                self.state.is_downloading = false;

                if is_error {
                    self.state.error = Some(message.clone());
                    self.state.last_error = Some(message);
                } else {
                    self.state.status = message;
                }

                ctx.request_repaint();
            }
        }
    }

    fn render_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 150.0);

            let button_text = self
                .localizer
                .lookup_single_language("download-button", None)
                .unwrap_or_else(|| "Download".to_string());

            let download_button = egui::Button::new(
                egui::RichText::new(button_text)
                    .size(BUTTON_FONT_SIZE)
                    .color(BUTTON_MAIN_TEXT),
            )
            .min_size(MIN_SIZE_BUTTON)
            .fill(PRIMARY_BUTTON_BG)
            .rounding(ROUNDING_BUTTON)
            .stroke(Stroke::new(1.0, BORDER_COLOR));

            if ui.add(download_button).clicked() {
                self.start_download(ctx);
            }

            let update_button = egui::Button::new(
                egui::RichText::new(
                    self.localizer
                        .lookup_single_language("update-ytdlp", None)
                        .unwrap_or_else(|| "Update yt-dlp".to_string()),
                )
                .size(BUTTON_FONT_SIZE)
                .color(BUTTON_MAIN_TEXT),
            )
            .min_size(MIN_SIZE_BUTTON)
            .fill(SECONDARY_BUTTON_BG)
            .rounding(ROUNDING_BUTTON)
            .stroke(Stroke::new(1.0, BORDER_COLOR));

            if ui.add(update_button).clicked() {
                self.update_ytdlp(ctx);
            }
        });
    }
}

impl eframe::App for YtdlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_ui(ctx);
    }
}
