use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadFormat {
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
