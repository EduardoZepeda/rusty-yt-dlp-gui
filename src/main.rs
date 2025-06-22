use eframe::egui;
use i18n_embed::DesktopLanguageRequester;
use std::process::{Command, Stdio};
use std::path::PathBuf;
use std::thread;

// Format selection
#[derive(PartialEq, Clone, Copy)]
enum DownloadFormat {
    MP4,
    MP3,
}

impl Default for DownloadFormat {
    fn default() -> Self {
        Self::MP4
    }
}

// App state
#[derive(Default)]
struct AppState {
    url: String,
    status: String,
    is_downloading: bool,
    output_path: Option<PathBuf>,
    progress: f32,
    download_speed: String,
    eta: String,
    last_error: Option<String>,
    format: DownloadFormat,
}

mod localizations;
use localizations::Localizations;

struct YtdlApp {
    state: AppState,
    localizer: Localizations,
    status_receiver: Option<std::sync::mpsc::Receiver<(bool, String)>>,
}

impl YtdlApp {
    fn new() -> Self {
        let mut localizer = Localizations::new();
        
        // This will load the user's preferred language
        let requested_languages = DesktopLanguageRequester::requested_languages();
        if let Some(lang) = requested_languages.first() {
            let _ = localizer.select(lang);
        }
        
        let mut state = AppState::default();
        state.status = localizer.lookup_single_language("status-ready", None)
            .unwrap_or_else(|| "Ready".to_string());
        
        Self {
            state,
            localizer,
            status_receiver: None,
        }
    }
}

impl eframe::App for YtdlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for download completion
        if let Some(rx) = &mut self.status_receiver {
            if let Ok((success, message)) = rx.try_recv() {
                self.state.is_downloading = false;
                if success {
                    self.state.status = message;
                    self.state.last_error = None;
                } else {
                    self.state.status = "Download failed".to_string();
                    self.state.last_error = Some(message);
                }
                self.status_receiver = None;
            }
        }
        
        // Update progress if downloading
        if self.state.is_downloading {
            // In a real app, you would parse the progress from yt-dlp output
            // For now, we'll just simulate some progress
            self.state.progress = (self.state.progress + 1.0).min(100.0);
        }
        // Get the language loader
        let loader = self.localizer.language_loader();
        
        // Get the localized strings
        let app_title = loader.lookup_single_language("app-title", None)
            .unwrap_or_else(|| "YouTube Downloader".to_string());
        let url_label = loader.lookup_single_language("url-label", None)
            .unwrap_or_else(|| "URL:".to_string());
        let url_placeholder = loader.lookup_single_language("url-placeholder", None)
            .unwrap_or_else(|| "Enter video URL".to_string());
        let button_text = loader.lookup_single_language("download-button", None)
            .unwrap_or_else(|| "Download".to_string());
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(&app_title);
            
            ui.add_space(20.0);
            
            // Format selection
            ui.horizontal(|ui| {
                let format_label = self.localizer.lookup_single_language("download-format", None)
                    .unwrap_or_else(|| "Download as:".to_string());
                ui.label(&format_label);
                
                let mp4_label = self.localizer.lookup_single_language("format-mp4", None)
                    .unwrap_or_else(|| "MP4 (Video)".to_string());
                ui.radio_value(&mut self.state.format, DownloadFormat::MP4, mp4_label);
                
                let mp3_label = self.localizer.lookup_single_language("format-mp3", None)
                    .unwrap_or_else(|| "MP3 (Audio only)".to_string());
                ui.radio_value(&mut self.state.format, DownloadFormat::MP3, mp3_label);
            });
            
            // URL input
            ui.horizontal(|ui| {
                let url_label = self.localizer.lookup_single_language("url-label", None)
                    .unwrap_or_else(|| "Video URL:".to_string());
                ui.label(&url_label);
                
                let response = ui.text_edit_singleline(&mut self.state.url);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.download();
                }
            });
            
            // Add some spacing
            ui.add_space(10.0);
            
            // Get the localized button text
            let button_text = self.localizer.lookup_single_language("download-button", None)
                .unwrap_or_else(|| "Download".to_string());
            
            // Disable the download button if a download is in progress
            if ui.add_enabled(!self.state.is_downloading, egui::Button::new(&button_text)).clicked() {
                self.download();
            }
            
            // Show status and progress
            ui.add_space(10.0);
            ui.label(&self.state.status);
            
            if self.state.is_downloading {
                // Show progress bar
                ui.add(egui::ProgressBar::new(self.state.progress as f32 / 100.0)
                    .text(format!("{:.1}% - {} - ETA: {}", 
                        self.state.progress, 
                        self.state.download_speed,
                        self.state.eta)));
            }
            
            // Show output path if available
            if let Some(path) = &self.state.output_path {
                ui.label(format!("Saved to: {}", path.display()));
            }
            
            // Show error if any
            if let Some(error) = &self.state.last_error {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
            }
        });
    }
}

impl YtdlApp {
    fn download(&mut self) {
        let url = self.state.url.trim().to_string();
        if url.is_empty() {
            self.state.status = self.localizer.lookup_single_language("error-no-url", None)
                .unwrap_or_else(|| "Error: Please enter a URL".to_string());
            self.state.last_error = Some("No URL provided".to_string());
            return;
        }

        // Check if yt-dlp is installed
        let ytdlp_cmd = match which::which("yt-dlp") {
            Ok(cmd) => cmd,
            Err(_) => {
                let msg = self.localizer.lookup_single_language("error-ytdlp-missing", None)
                    .unwrap_or_else(|| "Error: yt-dlp not found. Please install it first.".to_string());
                self.state.status = format!("Error: {}", msg);
                self.state.last_error = Some(msg);
                return;
            }
        };

        // Reset state
        self.state.is_downloading = true;
        self.state.progress = 0.0;
        self.state.download_speed = String::new();
        self.state.eta = String::new();
        self.state.last_error = None;
        self.state.status = self.localizer.lookup_single_language("status-downloading", None)
            .unwrap_or_else(|| "Preparing download...".to_string());
        
        // Create a channel for status updates
        let (tx, rx) = std::sync::mpsc::channel();
        
        // Clone the necessary data for the thread
        let url_clone = url.clone();
        
        // Format-specific arguments
        let format_args = match self.state.format {
            DownloadFormat::MP4 => vec![
                "-f", "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best",
                "--merge-output-format", "mp4",
            ],
            DownloadFormat::MP3 => vec![
                "-x",  // Extract audio
                "--audio-format", "mp3",
                "--audio-quality", "0",  // Best quality
            ],
        };
        
        // Output template
        let output_template = match self.state.format {
            DownloadFormat::MP4 => "%(title)s.%(ext)s",
            DownloadFormat::MP3 => "%(title)s.%(ext)s",
        };
        
        // Spawn a new thread for the download
        thread::spawn(move || {
            // Build the command with format-specific arguments
            let mut command = Command::new(ytdlp_cmd);
            
            // Common arguments
            command
                .arg("--newline")
                .arg("--no-simulate")
                .arg("--progress")
                .arg("--progress-template")
                .arg("[PROGRESS]%(progress._percent_str)s %(progress.speed)s ETA %(progress.eta)s")
                .arg("-o")
                .arg(output_template);
            
            // Add format-specific arguments
            for arg in format_args {
                command.arg(arg);
            }
            
            // Add the URL last
            command.arg(&url_clone);
            
            // Execute the command
            let output = command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .and_then(|child| child.wait_with_output());

            let result = match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if let Some(title) = stdout.lines().find(|line| !line.starts_with("[PROGRESS]")) {
                            (true, format!("Downloaded: {}", title.trim()))
                        } else {
                            (true, "Download completed successfully!".to_string())
                        }
                    } else {
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        (false, format!("Download failed: {}", error_msg))
                    }
                }
                Err(e) => {
                    (false, format!("Failed to start download: {}", e))
                }
            };
            
            // Send the result back to the main thread
            if let Err(e) = tx.send(result) {
                eprintln!("Failed to send download result: {}", e);
            }
        });
        
        // Store the receiver in the app
        self.status_receiver = Some(rx);
        self.state.status = format!("{} {}", 
            self.localizer.lookup_single_language("status-downloading", None)
                .unwrap_or_else(|| "Starting download:".to_string()),
            url
        );
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 200.0])
            .with_min_inner_size([300.0, 150.0])
            .with_title("YouTube Downloader"),
        ..Default::default()
    };
    
    eframe::run_native(
        "YouTube Downloader",
        options,
        Box::new(|_cc| Box::new(YtdlApp::new())),
    )
}
