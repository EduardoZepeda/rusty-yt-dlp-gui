use dirs;
use eframe::egui;
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

mod localizations;
use localizations::Localizations;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadFormat {
    MP4,
    MP3,
}

impl Default for DownloadFormat {
    fn default() -> Self {
        Self::MP4
    }
}

#[derive(Default)]
pub struct AppState {
    pub url: String,
    pub format: DownloadFormat,
    pub is_downloading: bool,
    pub progress: f32,
    pub status: String,
    pub error: Option<String>,
    pub last_error: Option<String>,
    pub download_speed: String,
    pub eta: String,
    pub output_path: Option<PathBuf>,
    pub download_dir: String,
}

struct YtdlApp {
    state: AppState,
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
    fn new() -> Self {
        // Create a new channel for status updates
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

    fn start_download(&mut self, ctx: &egui::Context) {
        if self.state.is_downloading {
            return;
        }

        if self.state.url.trim().is_empty() {
            self.state.error = Some("Please enter a URL".to_string());
            self.state.last_error = Some("No URL provided".to_string());
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

        thread::spawn(move || {
            let output = Command::new("yt-dlp")
                .arg("--newline")
                .arg("--progress")
                .arg("--no-check-certificate")
                .arg(if matches!(format, DownloadFormat::MP3) {
                    "-x"
                } else {
                    "-f"
                })
                .arg(if matches!(format, DownloadFormat::MP3) {
                    "--audio-format mp3 --audio-quality 0"
                } else {
                    "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best"
                })
                .arg(&url)
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let _ = tx.send((false, "Download complete".to_string()));
                    } else {
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        let _ = tx.send((true, error_msg.to_string()));
                    }
                }
                Err(e) => {
                    let _ = tx.send((true, e.to_string()));
                }
            }
        });

        // Request a repaint to update the UI
        ctx.request_repaint();
    }

    fn update_ytdlp(&mut self, ctx: &egui::Context) {
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

        thread::spawn(move || {
            let output = Command::new("yt-dlp")
                .arg("-U")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let _ =
                            tx.send((false, format!("yt-dlp updated successfully: {}", stdout)));
                    } else {
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        let _ = tx.send((true, format!("Failed to update yt-dlp: {}", error_msg)));
                    }
                }
                Err(e) => {
                    let _ = tx.send((true, format!("Failed to run yt-dlp: {}", e)));
                }
            }
        });

        // Request a repaint to update the UI
        ctx.request_repaint();
    }
}

impl eframe::App for YtdlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for status updates
        if let Some(receiver) = &mut self.status_receiver {
            while let Ok((is_error, message)) = receiver.try_recv() {
                // Check if this is a progress update (contains %)
                if message.contains('%') {
                    if let Some(percent_str) = message.split('%').next() {
                        if let Ok(percent) = percent_str.trim().parse::<f32>() {
                            self.state.progress = percent; // Store as percentage (0-100)

                            // Extract speed and ETA from the message
                            let parts: Vec<&str> = message.split_whitespace().collect();
                            if parts.len() >= 4 && parts[2] == "ETA:" {
                                self.state.download_speed = parts[1].to_string();
                                self.state.eta = parts[3].to_string();
                            }

                            self.state.status = message.clone();
                            // Request repaint to update progress
                            ctx.request_repaint();
                            continue;
                        }
                    }
                }

                // If not a progress update, handle as status message
                self.state.is_downloading = false;

                if is_error {
                    self.state.error = Some(message.clone());
                    self.state.last_error = Some(message);
                } else {
                    self.state.status = message;
                }

                // Request another repaint to show the final status
                ctx.request_repaint();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(
                self.localizer
                    .lookup_single_language("app-title", None)
                    .unwrap_or_else(|| "YouTube Downloader".to_string()),
            );

            ui.add_space(20.0);

            // URL input with larger size and border
            ui.label(
                self.localizer
                    .lookup_single_language("url-label", None)
                    .unwrap_or_else(|| "Video URL:".to_string()),
            );

            let response = egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(250, 250, 250))
                .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY))
                .rounding(4.0)
                .show(ui, |ui| {
                    ui.add_sized(
                        [ui.available_width(), 40.0], // Larger height for the input
                        egui::TextEdit::singleline(&mut self.state.url)
                            .hint_text(
                                self.localizer
                                    .lookup_single_language("url-placeholder", None)
                                    .unwrap_or_else(|| "Enter video URL".to_string()),
                            )
                            .font(egui::TextStyle::Body)
                            .font(egui::FontId::proportional(16.0)),
                    )
                })
                .inner;

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.start_download(ctx);
            }

            ui.add_space(10.0);

            // Format selection
            ui.horizontal(|ui| {
                ui.label(
                    self.localizer
                        .lookup_single_language("download-format", None)
                        .unwrap_or_else(|| "Download as:".to_string()),
                );

                let mp4_label = self
                    .localizer
                    .lookup_single_language("format-mp4", None)
                    .unwrap_or_else(|| "MP4 (Video)".to_string());
                let mp3_label = self
                    .localizer
                    .lookup_single_language("format-mp3", None)
                    .unwrap_or_else(|| "MP3 (Audio only)".to_string());

                ui.radio_value(&mut self.state.format, DownloadFormat::MP4, mp4_label);
                ui.radio_value(&mut self.state.format, DownloadFormat::MP3, mp3_label);
            });
            ui.add_space(20.0);
            // Download directory selection with improved UI
            ui.vertical(|ui| {
                ui.label(
                    self.localizer
                        .lookup_single_language("download-to", None)
                        .unwrap_or_else(|| "Download to:".to_string()),
                );

                // Container for the input and button
                ui.horizontal(|ui| {
                    // Directory input field with frame
                    egui::Frame::none()
                        .fill(ui.visuals().extreme_bg_color)
                        .rounding(4.0)
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .show(ui, |ui| {
                            ui.set_min_height(36.0);
                            let response = ui.add_sized(
                                [ui.available_width() - 100.0, 36.0],
                                egui::TextEdit::singleline(&mut self.state.download_dir)
                                    .hint_text("Select download directory")
                                    .frame(false)
                                    .margin(egui::vec2(8.0, 8.0)),
                            );
                            response
                        });

                    // Browse button with matching style
                    let button = egui::Button::new(
                        egui::RichText::new(
                            self.localizer
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
                        if let Some(path) = rfd::FileDialog::new()
                            .set_directory(
                                Path::new(&self.state.download_dir)
                                    .parent()
                                    .unwrap_or_else(|| Path::new(".")),
                            )
                            .pick_folder()
                        {
                            self.state.download_dir = path.to_string_lossy().to_string();
                        }
                    }
                });
            });

            ui.add_space(20.0);

            // Status in a frame
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(248, 248, 248))
                .rounding(8.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.add_space(10.0);

                        // Status text
                        let status_text = if let Some(error) = &self.state.last_error {
                            egui::RichText::new(format!("Error: {}", error))
                                .color(egui::Color32::RED)
                        } else {
                            egui::RichText::new(&self.state.status).color(egui::Color32::DARK_GRAY)
                        };
                        ui.label(status_text);

                        // Progress bar
                        if self.state.is_downloading {
                            ui.add_space(10.0);
                            let progress = self.state.progress / 100.0;
                            let progress_bar = egui::ProgressBar::new(progress)
                                .show_percentage()
                                .text(if progress > 0.0 {
                                    format!("{} - {}", self.state.download_speed, self.state.eta)
                                } else {
                                    self.localizer
                                        .lookup_single_language("status-downloading", None)
                                        .unwrap_or_else(|| "Starting download...".to_string())
                                });
                            ui.add(progress_bar);
                        }

                        // Output path
                        if let Some(path) = &self.state.output_path {
                            ui.add_space(10.0);
                            ui.label(format!("Saved to: {}", path.display()));
                        }

                        ui.add_space(10.0);
                    });
                });

            // Add flexible space to push buttons to bottom
            ui.add_space(ui.available_height() - 100.0);

            // Buttons container at the bottom
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 150.0);

                // Download button
                let button_text = self
                    .localizer
                    .lookup_single_language("download-button", None)
                    .unwrap_or_else(|| "Download".to_string());

                let download_button = egui::Button::new(
                    egui::RichText::new(button_text)
                        .size(16.0)
                        .color(egui::Color32::WHITE),
                )
                .min_size(egui::vec2(200.0, 50.0))
                .frame(true)
                .fill(egui::Color32::from_rgb(74, 144, 226))
                .rounding(8.0);

                let response = ui.add_enabled(!self.state.is_downloading, download_button);
                if response.clicked() {
                    self.download();
                }

                // Add some space between buttons
                ui.add_space(20.0);

                // Update button
                let update_text = self
                    .localizer
                    .lookup_single_language("update-button", None)
                    .unwrap_or_else(|| "Update yt-dlp".to_string());

                let update_button = egui::Button::new(
                    egui::RichText::new(update_text)
                        .size(16.0)
                        .color(egui::Color32::DARK_GRAY),
                )
                .min_size(egui::vec2(200.0, 50.0))
                .frame(true)
                .fill(egui::Color32::LIGHT_GRAY)
                .rounding(6.0);

                if ui
                    .add_enabled(!self.state.is_downloading, update_button)
                    .clicked()
                {
                    self.update_ytdlp(ctx);
                }
            });

            // Add some space at the bottom
            ui.add_space(10.0);
        });
    }

    // This method is now implemented in the YtdlApp implementation
    // The implementation is moved to the impl YtdlApp block
}

impl YtdlApp {
    fn download(&mut self) {
        let url = self.state.url.trim().to_string();
        if url.is_empty() {
            self.state.status = self
                .localizer
                .lookup_single_language("error-no-url", None)
                .unwrap_or_else(|| "Error: Please enter a URL".to_string());
            self.state.last_error = Some("No URL provided".to_string());
            return;
        }

        // Check if yt-dlp is installed
        let ytdlp_cmd = match which::which("yt-dlp") {
            Ok(cmd) => cmd,
            Err(_) => {
                let msg = self
                    .localizer
                    .lookup_single_language("error-ytdlp-missing", None)
                    .unwrap_or_else(|| {
                        "Error: yt-dlp not found. Please install it first.".to_string()
                    });
                self.state.status = format!("Error: {}", msg);
                self.state.last_error = Some(msg);
                return;
            }
        };

        // Validate download directory
        let download_dir = if self.state.download_dir.trim().is_empty() {
            std::env::current_dir().unwrap_or_default()
        } else {
            PathBuf::from(&self.state.download_dir)
        };

        if !download_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&download_dir) {
                self.state.status = format!("Error creating directory: {}", e);
                self.state.last_error = Some(format!("Directory error: {}", e));
                return;
            }
        }

        // Reset state
        self.state.is_downloading = true;
        self.state.progress = 0.0;
        self.state.download_speed = String::new();
        self.state.eta = String::new();
        self.state.last_error = None;
        self.state.error = None;
        self.state.output_path = None;
        self.state.status = self
            .localizer
            .lookup_single_language("status-downloading", None)
            .unwrap_or_else(|| "Preparing download...".to_string());

        println!("Starting download to: {}", download_dir.display()); // Debug log

        // Create a channel for status updates
        let (tx, rx) = std::sync::mpsc::channel();

        // Clone the necessary data for the thread
        let url_clone = url.clone();
        let format = self.state.format;
        let download_dir_clone = download_dir.clone();

        // Spawn a new thread for the download
        thread::spawn(move || {
            // Build the command
            let mut command = std::process::Command::new(ytdlp_cmd);

            // Common arguments
            let output_template = download_dir_clone
                .join("%(title)s.%(ext)s")
                .to_string_lossy()
                .to_string();
            println!("Output template: {}", output_template); // Debug log

            command
                .arg("--newline")
                .arg("--progress")
                .arg("--no-simulate")
                .arg("--progress-template")
                .arg("PROGRESS:%(progress._percent_str)s|%(progress.speed)s|%(progress.eta)s")
                .arg("-o")
                .arg(&output_template);

            println!("Command: {:?}", command); // Debug log

            // Add format-specific arguments
            match format {
                DownloadFormat::MP4 => {
                    command
                        .arg("-f")
                        .arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best")
                        .arg("--merge-output-format")
                        .arg("mp4");
                }
                DownloadFormat::MP3 => {
                    command
                        .arg("-x")
                        .arg("--audio-format")
                        .arg("mp3")
                        .arg("--audio-quality")
                        .arg("0");
                }
            }

            // Add the URL last
            command.arg(&url_clone);

            // Execute the command and capture the output in real-time
            let output = command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    // Read stderr in a separate thread to avoid deadlocks
                    let stderr = child.stderr.take().expect("Failed to capture stderr");
                    let tx_err = tx.clone();

                    std::thread::spawn(move || {
                        use std::io::{BufRead, BufReader};
                        let reader = BufReader::new(stderr);

                        for line in reader.lines() {
                            if let Ok(line) = line {
                                if line.starts_with("PROGRESS:") {
                                    let parts: Vec<&str> = line[9..].split('|').collect();
                                    if parts.len() >= 3 {
                                        let percent = parts[0]
                                            .trim()
                                            .trim_end_matches('%')
                                            .parse::<f32>()
                                            .unwrap_or(0.0);
                                        let speed = parts[1].trim().to_string();
                                        let eta = parts[2].trim().to_string();

                                        if let Err(e) = tx_err.send((
                                            false,
                                            format!("{:.1}% - {} ETA: {}", percent, speed, eta),
                                        )) {
                                            eprintln!("Failed to send progress update: {}", e);
                                            break;
                                        }
                                    }
                                } else if line.contains("%") || line.contains("ETA") {
                                    // Try to parse progress from other progress lines
                                    if let Some(percent_start) = line.find('%') {
                                        let percent_part = &line[..percent_start];
                                        if let Ok(percent) = percent_part.trim().parse::<f32>() {
                                            let speed = line
                                                .split_whitespace()
                                                .nth(1)
                                                .unwrap_or("")
                                                .to_string();
                                            let eta = line
                                                .split_whitespace()
                                                .nth(3)
                                                .unwrap_or("")
                                                .to_string();

                                            if let Err(e) = tx_err.send((
                                                false,
                                                format!("{:.1}% - {} ETA: {}", percent, speed, eta),
                                            )) {
                                                eprintln!("Failed to send progress update: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });

                    child.wait_with_output()
                });

            let result = match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    println!("Command status: {}", output.status);
                    println!("stdout: {}", stdout);
                    println!("stderr: {}", stderr);

                    if output.status.success() {
                        if let Some(title) =
                            stdout.lines().find(|line| !line.starts_with("PROGRESS:"))
                        {
                            (false, format!("Downloaded: {}", title.trim()))
                        } else {
                            (false, "Download completed successfully!".to_string())
                        }
                    } else {
                        let error_msg = if stderr.is_empty() {
                            format!("Command failed with status: {}", output.status)
                        } else {
                            stderr.to_string()
                        };
                        (true, format!("Download failed: {}", error_msg))
                    }
                }
                Err(e) => (true, format!("Failed to start download: {}", e)),
            };

            // Send the final result back to the main thread
            if let Err(e) = tx.send(result) {
                eprintln!("Failed to send download result: {}", e);
            }
        });

        // Store the receiver in the app
        self.status_receiver = Some(rx);
        self.state.status = format!(
            "{} {}",
            self.localizer
                .lookup_single_language("status-downloading", None)
                .unwrap_or_else(|| "Starting download:".to_string()),
            url
        );
    }
}

fn main() -> eframe::Result<()> {
    // Set default download directory to user's downloads folder
    let default_download_dir = dirs::download_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .to_string_lossy()
        .to_string();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 550.0]) // Slightly larger window to fit directory selection
            .with_min_inner_size([600.0, 450.0])
            .with_title("YouTube Downloader"),
        ..Default::default()
    };

    // Initialize app state with default download directory
    let app = YtdlApp {
        state: AppState {
            download_dir: default_download_dir,
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "YouTube Downloader",
        options,
        Box::new(|cc| {
            // Set light theme
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Box::new(app)
        }),
    )
}
