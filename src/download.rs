use std::fs;
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;
use std::io::Write;
use reqwest::blocking::get;
use std::os::unix::fs::PermissionsExt;

use crate::models::DownloadFormat;

const YT_DLP_BINARY: &str = "yt-dlp";
const YT_DLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp";

fn get_local_ytdlp_path() -> std::path::PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| "./".into());
    path.push("ytdl-gui");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push(YT_DLP_BINARY);
    path
}

fn ensure_ytdlp_exists() -> Result<String, String> {
    let local_path = get_local_ytdlp_path();
    
    // If binary doesn't exist or is older than 7 days, download it
    let needs_download = if local_path.exists() {
        let metadata = fs::metadata(&local_path).map_err(|e| e.to_string())?;
        let modified = metadata.modified().map_err(|e| e.to_string())?;
        let age = std::time::SystemTime::now()
            .duration_since(modified)
            .unwrap_or_else(|_| std::time::Duration::from_secs(60 * 60 * 24 * 8)); // Default to 8 days if time went backwards
        age > std::time::Duration::from_secs(60 * 60 * 24 * 7) // 7 days
    } else {
        true
    };

    if needs_download {
        download_ytdlp(&local_path)?;
    } else {
        // Make sure it's executable
        let perms = fs::metadata(&local_path)
            .map_err(|e| e.to_string())?
            .permissions();
        if perms.mode() & 0o111 == 0 {
            let mut perms = perms;
            perms.set_mode(0o755); // rwxr-xr-x
            fs::set_permissions(&local_path, perms).map_err(|e| e.to_string())?;
        }
    }

    local_path.to_str().ok_or_else(|| "Invalid path".to_string()).map(|s| s.to_string())
}

fn download_ytdlp(path: &std::path::Path) -> Result<(), String> {
    let response = get(YT_DLP_URL).map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Failed to download yt-dlp: {}", response.status()));
    }
    
    let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
    let content = response.bytes().map_err(|e| e.to_string())?;
    file.write_all(&content).map_err(|e| e.to_string())?;
    
    // Make it executable
    let mut perms = fs::metadata(path).map_err(|e| e.to_string())?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x
    fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
    
    Ok(())
}

pub fn start_download(
    url: String,
    format: DownloadFormat,
    download_dir: String,
    tx: Sender<(bool, String)>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let ytdlp_path = match ensure_ytdlp_exists() {
            Ok(path) => path,
            Err(e) => {
                let _ = tx.send((true, format!("Failed to get yt-dlp: {}", e)));
                return;
            }
        };

        let mut cmd = Command::new(&ytdlp_path);
        
        cmd.arg("--newline")
           .arg("--progress")
           .arg("--no-check-certificate");
        
        if matches!(format, DownloadFormat::MP3) {
            cmd.arg("-x")
               .arg("--audio-format")
               .arg("mp3")
               .arg("--audio-quality")
               .arg("0");
        } else {
            cmd.arg("-f")
               .arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best");
        }
        
        // Set output directory and template
        cmd.arg("-P")
           .arg(&download_dir)
           .arg("-o")
           .arg("%(title)s.%(ext)s")
           .arg("--newline")
           .arg("--progress")
           .arg("--console-title")
           .arg("--no-simulate")
           .arg("--progress-template")
           .arg("[download] %(progress._percent_str)s of %(progress._total_bytes_str)s at %(progress._speed_str)s ETA %(progress._eta_str)s")
           .arg(&url);
        
        // Spawn the command with piped output
        let mut child = match cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn() {
                Ok(child) => child,
                Err(e) => {
                    let _ = tx.send((true, format!("Failed to start yt-dlp: {}", e)));
                    return;
                }
            };
        
        // Read stdout and stderr in a separate thread
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let tx_stdout = tx.clone();
        let tx_stderr = tx.clone();
        
        // Handle stdout (progress updates)
        let stdout_handle = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        println!("STDOUT: {}", line);
                        // Forward all progress-related lines
                        if line.starts_with("[download]") || line.contains("ETA") {
                            // Send the update and immediately request a repaint
                            if let Err(e) = tx_stdout.send((false, line)) {
                                println!("Failed to send progress update: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error reading from stdout: {}", e);
                    }
                }
            }
        });
        
        // Handle stderr (errors)
        let stderr_handle = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let line = line.trim().to_string();
                        if !line.is_empty() {
                            println!("STDERR: {}", line);
                            if let Err(e) = tx_stderr.send((true, line)) {
                                println!("Failed to send error message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error reading from stderr: {}", e);
                    }
                }
            }
        });
        
        // Wait for the process to complete
        let status = child.wait();
        
        // Wait for the output handlers to finish
        let _ = stdout_handle.join();
        let _ = stderr_handle.join();
        
        match status {
            Ok(exit_status) => {
                if exit_status.success() {
                    let _ = tx.send((false, "Download complete".to_string()));
                } else {
                    let _ = tx.send((true, format!("Process exited with: {}", exit_status)));
                }
            }
            Err(e) => {
                let _ = tx.send((true, format!("Failed to wait for process: {}", e)));
            }
        }
    })
}

pub fn update_ytdlp(tx: Sender<(bool, String)>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let local_path = get_local_ytdlp_path();
        
        // Remove the existing binary to force a fresh download
        if local_path.exists() {
            if let Err(e) = fs::remove_file(&local_path) {
                let _ = tx.send((true, format!("Failed to remove existing yt-dlp: {}", e)));
                return;
            }
        }
        
        // This will download a fresh copy
        match ensure_ytdlp_exists() {
            Ok(_) => {
                let _ = tx.send((false, "yt-dlp updated".to_string()));
            }
            Err(e) => {
                let _ = tx.send((true, format!("Failed to update yt-dlp: {}", e)));
            }
        }
    })
}
