use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use crate::models::DownloadFormat;

pub fn start_download(
    url: String,
    format: DownloadFormat,
    tx: Sender<(bool, String)>,
) -> thread::JoinHandle<()> {
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
    })
}

pub fn update_ytdlp(tx: Sender<(bool, String)>) -> thread::JoinHandle<()> {
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
                    let _ = tx.send((false, format!("yt-dlp updated successfully: {}", stdout)));
                } else {
                    let error_msg = String::from_utf8_lossy(&output.stderr);
                    let _ = tx.send((true, format!("Failed to update yt-dlp: {}", error_msg)));
                }
            }
            Err(e) => {
                let _ = tx.send((true, format!("Failed to run yt-dlp: {}", e)));
            }
        }
    })
}
