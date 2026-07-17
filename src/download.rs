use reqwest::blocking::get;
use std::fs;
use std::io::Write;
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;

use crate::models::DownloadFormat;

// ============================================================================
// Platform Abstraction (Martin Fowler: Replace Conditional with Polymorphism)
// ============================================================================

/// Platform-specific configuration for yt-dlp binary management
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    /// Name of the yt-dlp binary (e.g., "yt-dlp" or "yt-dlp.exe")
    pub binary_name: String,
    /// Download URL for the binary
    pub download_url: String,
    /// Whether this platform requires explicit execute permission
    pub needs_execute_permission: bool,
}

impl PlatformConfig {
    /// Detect current platform and return appropriate configuration
    #[allow(unreachable_code)]
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            return Self::windows();
        }
        #[cfg(target_os = "macos")]
        {
            return Self::macos();
        }
        #[cfg(any(target_os = "linux", all(unix, not(target_os = "macos"))))]
        {
            return Self::unix();
        }

        // Fallback to Unix defaults
        Self::unix()
    }

    pub fn windows() -> Self {
        Self {
            binary_name: "yt-dlp.exe".to_string(),
            download_url: "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
                .to_string(),
            needs_execute_permission: false,
        }
    }

    pub fn macos() -> Self {
        // macOS uses the same binary as Linux
        Self {
            binary_name: "yt-dlp".to_string(),
            download_url: "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
                .to_string(),
            needs_execute_permission: true,
        }
    }

    pub fn unix() -> Self {
        Self {
            binary_name: "yt-dlp".to_string(),
            download_url: "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
                .to_string(),
            needs_execute_permission: true,
        }
    }
}

/// Trait for platform-specific file operations (Strategy Pattern)
trait PlatformOps: Send + Sync {
    fn make_executable(&self, path: &std::path::Path) -> Result<(), String>;
}

struct UnixPlatformOps;
struct WindowsPlatformOps;

impl PlatformOps for UnixPlatformOps {
    fn make_executable(&self, path: &std::path::Path) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;

        let perms = fs::metadata(path).map_err(|e| e.to_string())?.permissions();

        // Only set execute bit if it's not already set
        if perms.mode() & 0o111 == 0 {
            let mut perms = perms;
            perms.set_mode(0o755); // rwxr-xr-x
            fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

impl PlatformOps for WindowsPlatformOps {
    fn make_executable(&self, _path: &std::path::Path) -> Result<(), String> {
        // Windows doesn't need execute permissions - the file system handles this
        // The .exe extension alone makes it executable
        Ok(())
    }
}

/// Get the appropriate platform operations (Factory Pattern)
#[allow(unreachable_code)]
fn get_platform_ops() -> &'static dyn PlatformOps {
    #[cfg(target_os = "windows")]
    {
        return &WindowsPlatformOps;
    }

    &UnixPlatformOps
}

// ============================================================================
// JavaScript Runtime Management (yt-dlp >= 2025.11.12 requires a JS runtime)
// ============================================================================

/// Platform-specific configuration for the Deno JS runtime
struct DenoConfig {
    binary_name: String,
    download_url: String,
}

impl DenoConfig {
    #[allow(unreachable_code)]
    fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            return Self {
                binary_name: "deno.exe".to_string(),
                download_url: "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip".to_string(),
            };
        }
        #[cfg(target_os = "macos")]
        {
            #[cfg(target_arch = "aarch64")]
            {
                return Self {
                    binary_name: "deno".to_string(),
                    download_url: "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-apple-darwin.zip".to_string(),
                };
            }
            #[cfg(not(target_arch = "aarch64"))]
            {
                return Self {
                    binary_name: "deno".to_string(),
                    download_url: "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-apple-darwin.zip".to_string(),
                };
            }
        }
        #[cfg(any(target_os = "linux", all(unix, not(target_os = "macos"))))]
        {
            return Self {
                binary_name: "deno".to_string(),
                download_url: "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip".to_string(),
            };
        }

        // Fallback
        Self {
            binary_name: "deno".to_string(),
            download_url: "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip".to_string(),
        }
    }
}

/// Get the local path where the Deno binary is stored (same dir as yt-dlp)
fn get_local_deno_path() -> std::path::PathBuf {
    let deno_config = DenoConfig::current();
    let mut path = dirs::config_dir().unwrap_or_else(|| "./".into());
    path.push("ytdl-gui");
    path.push(deno_config.binary_name);
    path
}

/// Ensure Deno exists locally, downloading it if necessary.
/// Returns the path to the deno binary.
fn ensure_deno_exists() -> Result<String, String> {
    let deno_config = DenoConfig::current();
    let platform_ops = get_platform_ops();
    let local_path = get_local_deno_path();

    if !local_path.exists() {
        download_deno(&local_path, &deno_config, platform_ops)?;
    } else {
        // Make sure it's executable
        platform_ops.make_executable(&local_path)?;
    }

    local_path
        .to_str()
        .ok_or_else(|| "Invalid deno path".to_string())
        .map(|s| s.to_string())
}

/// Download Deno from GitHub releases (zip) and extract the binary
fn download_deno(
    target_path: &std::path::Path,
    deno_config: &DenoConfig,
    platform_ops: &dyn PlatformOps,
) -> Result<(), String> {
    let response =
        reqwest::blocking::get(&deno_config.download_url).map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download Deno: {}",
            response.status()
        ));
    }

    let zip_bytes = response.bytes().map_err(|e| e.to_string())?;

    // Write zip to a temp file, then extract with unzip
    let zip_path = target_path.with_extension("zip");
    fs::write(&zip_path, &zip_bytes).map_err(|e| e.to_string())?;

    // Extract the deno binary from the zip
    let parent_dir = target_path.parent().ok_or("No parent directory")?;
    let extract_status = Command::new("unzip")
        .arg("-o") // overwrite
        .arg("-j") // flatten (extract to single dir)
        .arg(&zip_path)
        .arg("-d")
        .arg(parent_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .map_err(|e| format!("Failed to run unzip. Is 'unzip' installed? ({})", e))?;

    // Clean up the zip file
    let _ = fs::remove_file(&zip_path);

    if !extract_status.success() {
        return Err("Failed to extract Deno zip archive".to_string());
    }

    // After unzip -j, the binary lands directly in parent_dir.
    // Move it to the exact target_path if names differ.
    let deno_config = DenoConfig::current();
    let extracted_path = parent_dir.join(&deno_config.binary_name);
    if extracted_path != target_path {
        fs::rename(&extracted_path, target_path).map_err(|e| {
            // Fallback: copy then remove
            let _ = fs::copy(&extracted_path, target_path);
            let _ = fs::remove_file(&extracted_path);
            e.to_string()
        })?;
    }

    // Make executable on Unix
    platform_ops.make_executable(target_path)?;

    Ok(())
}

// ============================================================================
// Main Logic (Extract Method - separate concerns)
// ============================================================================

fn get_local_ytdlp_path() -> std::path::PathBuf {
    let platform = PlatformConfig::current();
    let mut path = dirs::config_dir().unwrap_or_else(|| "./".into());
    path.push("ytdl-gui");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push(platform.binary_name);
    path
}

fn ensure_ytdlp_exists() -> Result<String, String> {
    let platform = PlatformConfig::current();
    let platform_ops = get_platform_ops();
    let local_path = get_local_ytdlp_path();

    // If binary doesn't exist or is older than 7 days, download it
    let needs_download = if local_path.exists() {
        let metadata = fs::metadata(&local_path).map_err(|e| e.to_string())?;
        let modified = metadata.modified().map_err(|e| e.to_string())?;
        let age = std::time::SystemTime::now()
            .duration_since(modified)
            .unwrap_or_else(|_| std::time::Duration::from_secs(60 * 60 * 24 * 8));
        age > std::time::Duration::from_secs(60 * 60 * 24 * 7)
    } else {
        true
    };

    if needs_download {
        download_ytdlp(&local_path, &platform, platform_ops)?;
    } else {
        // Make sure it's executable (if needed by platform)
        if platform.needs_execute_permission {
            platform_ops.make_executable(&local_path)?;
        }
    }

    local_path
        .to_str()
        .ok_or_else(|| "Invalid path".to_string())
        .map(|s| s.to_string())
}

fn download_ytdlp(
    path: &std::path::Path,
    platform: &PlatformConfig,
    platform_ops: &dyn PlatformOps,
) -> Result<(), String> {
    let response = reqwest::blocking::get(&platform.download_url).map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Failed to download yt-dlp: {}", response.status()));
    }

    let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
    let content = response.bytes().map_err(|e| e.to_string())?;
    file.write_all(&content).map_err(|e| e.to_string())?;

    // Apply platform-specific executable permissions
    if platform.needs_execute_permission {
        platform_ops.make_executable(path)?;
    }

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

        // Ensure Deno is available for yt-dlp YouTube player challenges
        let _deno_path = match ensure_deno_exists() {
            Ok(path) => path,
            Err(e) => {
                let _ = tx.send((true, format!("Failed to get Deno JS runtime: {}. yt-dlp requires a JS runtime to download YouTube videos.", e)));
                return;
            }
        };

        // Build PATH that includes the ytdl-gui config dir (where deno lives)
        let config_dir = dirs::config_dir().unwrap_or_else(|| "./".into());
        let ytdl_gui_dir = config_dir.join("ytdl-gui");
        let config_dir_str = ytdl_gui_dir.to_string_lossy().to_string();

        // Prepend our config dir to the system PATH so yt-dlp finds deno
        let augmented_path = if let Ok(system_path) = std::env::var("PATH") {
            format!("{}:{}", config_dir_str, system_path)
        } else {
            config_dir_str.clone()
        };

        let mut cmd = Command::new(&ytdlp_path);

        // Point yt-dlp at deno via --js-runtimes (belt AND suspenders with PATH)
        let deno_path = ytdl_gui_dir.join("deno");
        cmd.arg("--js-runtimes")
           .arg(format!("deno:{}", deno_path.display()));

        cmd.arg("--newline")
            .arg("--progress")
            .arg("--no-check-certificate")
            .env("PATH", &augmented_path);

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
        let mut child = match cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
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
                        if line.starts_with("[download]") || line.contains("ETA") {
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
