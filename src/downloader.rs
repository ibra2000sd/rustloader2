use crate::error::{AppError, NetworkErrorKind};
use crate::utils::{format_output_path, initialize_download_dir, validate_bitrate, validate_path_safety, validate_time_format, validate_url};
use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use colored::*;
use dirs_next as dirs;
use humansize::{format_size, BINARY};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use notify_rust::Notification;
use once_cell::sync::Lazy;
use rand::{thread_rng, Rng};
use regex::Regex;
use ring::{digest, hmac};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as AsyncCommand;
use tokio::time::sleep;

// We don't need to re-export these types since they're not actually used in this module
// The imports are available directly from download_manager when needed

const FREE_MP3_BITRATE: &str = "128K";

static FFMPEG_AVAILABLE: Lazy<bool> = Lazy::new(|| {
    if std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return true;
    }
    
    let common_paths = vec![
        "/usr/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/opt/homebrew/bin/ffmpeg",
        "/snap/bin/ffmpeg",
        "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
        "C:\\ffmpeg\\bin\\ffmpeg.exe",
    ];
    
    for path in common_paths {
        if Path::new(path).exists() {
            return true;
        }
    }
    
    #[cfg(target_os = "windows")]
    let which_cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let which_cmd = "which";

    if let Ok(output) = std::process::Command::new(which_cmd).arg("ffmpeg").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && std::path::Path::new(&path).exists() {
                return true;
            }
        }
    }
    
    false
});

/// Constants for network resilience
const MAX_RETRIES: usize = 5;
const INITIAL_RETRY_DELAY_MS: u64 = 1000; // Start with 1 second
const MAX_RETRY_DELAY_MS: u64 = 60000; // Max 1 minute
const NETWORK_CHECK_TIMEOUT_MS: u64 = 5000; // 5 seconds for network check
const STALL_DETECTION_SECONDS: u64 = 30; // Consider download stalled after 30s with no progress

/// Constants for memory management
const BUFFER_SIZE: usize = 64 * 1024; // 64 KB buffer size for optimal streaming
const SPEED_SAMPLE_LIMIT: usize = 10; // Limit number of speed samples to control memory growth
const SPEED_SAMPLE_INTERVAL_MS: u64 = 300; // Only sample speed every 300ms to reduce memory pressure
const MEMORY_CLEANUP_INTERVAL_SECS: u64 = 60; // Cleanup unused memory every 60 seconds

/// Enhanced download progress tracking with network resilience and memory optimization features
struct DownloadProgress {
    last_update: Mutex<Instant>,
    downloaded_bytes: AtomicU64,
    total_bytes: AtomicU64,
    download_speed: Mutex<f64>,
    last_speed_samples: Mutex<Vec<f64>>,
    download_active: AtomicBool,
    last_progress_time: Mutex<Instant>,
    resumable: AtomicBool,
    retry_count: AtomicU64,
    last_memory_cleanup: Mutex<Instant>,
    download_start_time: Mutex<Instant>,
}

impl DownloadProgress {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            last_update: Mutex::new(now),
            downloaded_bytes: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            download_speed: Mutex::new(0.0),
            last_speed_samples: Mutex::new(Vec::with_capacity(SPEED_SAMPLE_LIMIT)),
            download_active: AtomicBool::new(true),
            last_progress_time: Mutex::new(now),
            resumable: AtomicBool::new(true),
            retry_count: AtomicU64::new(0),
            last_memory_cleanup: Mutex::new(now),
            download_start_time: Mutex::new(now),
        }
    }

    /// Update download progress and speed metrics with memory optimization
    fn update(&self, downloaded: u64, total: u64) {
        let current_downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        
        // Use saturating_sub to avoid manual overflow checking
        let bytes_diff = downloaded.saturating_sub(current_downloaded);
        
        self.downloaded_bytes.store(downloaded, Ordering::SeqCst);
        self.total_bytes.store(total, Ordering::SeqCst);

        // Mark download as active and update progress time
        self.download_active.store(true, Ordering::SeqCst);
        *self.last_progress_time.lock().unwrap() = Instant::now();

        let now = Instant::now();
        
        // Periodically perform memory cleanup during long downloads
        let should_cleanup = {
            let mut last_cleanup = self.last_memory_cleanup.lock().unwrap();
            let cleanup_needed = now.duration_since(*last_cleanup).as_secs() >= MEMORY_CLEANUP_INTERVAL_SECS;
            if cleanup_needed {
                *last_cleanup = now;
            }
            cleanup_needed
        };
        
        if should_cleanup {
            debug!("Performing periodic memory cleanup during download");
            self.cleanup_unused_memory();
        }

        // Only update speed calculations at defined intervals to reduce CPU/memory usage
        let mut last_update = self.last_update.lock().unwrap();
        let time_diff = now.duration_since(*last_update).as_millis();

        if time_diff >= SPEED_SAMPLE_INTERVAL_MS as u128 && bytes_diff > 0 {
            let mut last_speed_samples = self.last_speed_samples.lock().unwrap();
            let mut speed = self.download_speed.lock().unwrap();

            let current_speed = bytes_diff as f64 / (time_diff as f64 / 1000.0);
            
            // Add the new sample and maintain fixed size to prevent memory growth
            last_speed_samples.push(current_speed);
            if last_speed_samples.len() > SPEED_SAMPLE_LIMIT {
                last_speed_samples.remove(0);
            }
            
            // Calculate average speed only if we have samples
            if !last_speed_samples.is_empty() {
                let sum: f64 = last_speed_samples.iter().sum();
                *speed = sum / last_speed_samples.len() as f64;
            }

            *last_update = now;
        }
    }
    
    /// Cleanup unused memory to prevent leaks during long downloads
    fn cleanup_unused_memory(&self) {
        // Get download duration to adjust cleanup intensity based on duration
        let download_duration_secs = {
            let start_time = self.download_start_time.lock().unwrap();
            start_time.elapsed().as_secs()
        };
        
        // For longer downloads, be more aggressive with memory cleanup
        let cleanup_intensity = if download_duration_secs > 3600 {
            // For downloads longer than 1 hour
            "high"
        } else if download_duration_secs > 600 {
            // For downloads longer than 10 minutes
            "medium"
        } else {
            "low"
        };
        
        debug!("Performing memory cleanup with {} intensity after {} seconds of download", 
              cleanup_intensity, download_duration_secs);
        
        // Remove excess capacity from speed samples vector
        let mut samples = self.last_speed_samples.lock().unwrap();
        
        let threshold = match cleanup_intensity {
            "high" => SPEED_SAMPLE_LIMIT, // Very aggressive for long downloads
            "medium" => SPEED_SAMPLE_LIMIT * 2,
            _ => SPEED_SAMPLE_LIMIT * 3
        };
        
        if samples.capacity() > threshold {
            debug!("Shrinking speed samples vector from capacity {} to {}", samples.capacity(), SPEED_SAMPLE_LIMIT);
            let current_samples = samples.clone();
            *samples = current_samples;
            samples.shrink_to_fit();
        }
        
        // Force drop any large internal buffers
        drop(samples);
        
        // For high intensity cleanups, also call the system allocator
        if cleanup_intensity == "high" {
            debug!("Requesting system memory optimization for long-running download ({}s)", download_duration_secs);
            // On some platforms we could potentially make system calls to release memory
            // but this is platform specific. For now we'll rely on Rust's allocator.
        }
    }

    /// Detect if the download has stalled (no progress for a specified time)
    fn is_stalled(&self) -> bool {
        let last_progress = self.last_progress_time.lock().unwrap();
        let elapsed = last_progress.elapsed().as_secs();
        
        // If we've received no data for STALL_DETECTION_SECONDS, consider it stalled
        elapsed > STALL_DETECTION_SECONDS && self.download_active.load(Ordering::SeqCst)
    }

    /// Reset status for retry with memory optimization
    fn prepare_for_retry(&self) {
        // Increment retry counter
        self.retry_count.fetch_add(1, Ordering::SeqCst);
        
        // Reset speed samples and other metrics if needed
        let mut speed_samples = self.last_speed_samples.lock().unwrap();
        speed_samples.clear();
        
        // Prevent memory leaks by releasing excess capacity
        if speed_samples.capacity() > SPEED_SAMPLE_LIMIT {
            speed_samples.shrink_to_fit();
        }
        
        // Mark download as inactive while retrying
        self.download_active.store(false, Ordering::SeqCst);
        
        // Reset memory cleanup timer
        *self.last_memory_cleanup.lock().unwrap() = Instant::now();
    }
    
    /// Get current retry count
    fn get_retry_count(&self) -> u64 {
        self.retry_count.load(Ordering::SeqCst)
    }
    
    /// Calculate retry delay with exponential backoff
    fn get_retry_delay_ms(&self) -> u64 {
        let retry = self.get_retry_count();
        // Exponential backoff with jitter
        let base_delay = INITIAL_RETRY_DELAY_MS * 2u64.saturating_pow(retry as u32);
        let jitter = thread_rng().gen_range(0..=500); // Random 0-500ms jitter
        
        // Cap at maximum delay
        std::cmp::min(base_delay + jitter, MAX_RETRY_DELAY_MS)
    }
    
    /// Mark download as resumable or not
    fn set_resumable(&self, resumable: bool) {
        self.resumable.store(resumable, Ordering::SeqCst);
    }
    
    /// Check if download is resumable
    fn is_resumable(&self) -> bool {
        self.resumable.load(Ordering::SeqCst)
    }

    fn get_percentage(&self) -> u64 {
        let downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);
        if total == 0 { return 0; }
        (downloaded as f64 / total as f64 * 100.0) as u64
    }

    fn get_speed(&self) -> f64 {
        *self.download_speed.lock().unwrap()
    }

    fn get_eta(&self) -> Option<Duration> {
        let downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);
        let speed = self.get_speed();

        if speed <= 0.0 || downloaded >= total {
            return None;
        }

        let remaining_bytes = total - downloaded;
        let seconds_remaining = remaining_bytes as f64 / speed;
        Some(Duration::from_secs_f64(seconds_remaining))
    }

    fn format_eta(&self) -> String {
        match self.get_eta() {
            Some(duration) => {
                let total_secs = duration.as_secs();
                let hours = total_secs / 3600;
                let minutes = (total_secs % 3600) / 60;
                let seconds = total_secs % 60;

                if hours > 0 {
                    format!("{}h {}m {}s", hours, minutes, seconds)
                } else if minutes > 0 {
                    format!("{}m {}s", minutes, seconds)
                } else {
                    format!("{}s", seconds)
                }
            }
            None => "Calculating...".to_string(),
        }
    }

    fn format_speed(&self) -> String {
        let speed = self.get_speed();
        if speed <= 0.0 {
            return "Calculating...".to_string();
        }
        format!("{}/s", format_size(speed as u64, BINARY))
    }

    fn format_file_size(&self) -> String {
        let downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);

        if total == 0 {
            return format!("{} / Unknown", format_size(downloaded, BINARY));
        }

        format!("{} / {}", format_size(downloaded, BINARY), format_size(total, BINARY))
    }
    
    /// Format a status message showing retry information
    fn format_retry_status(&self) -> String {
        let retry = self.get_retry_count();
        let delay_ms = self.get_retry_delay_ms();
        
        format!("Retry {}/{} (waiting {}s)...", 
            retry, 
            MAX_RETRIES, 
            (delay_ms as f64 / 1000.0).ceil()
        )
    }
}

struct DownloadCounter {
    today_count: u32,
    date: String,
    max_daily_downloads: u32,
}

impl DownloadCounter {
    fn new() -> Self {
        Self {
            today_count: 0,
            date: Local::now().format("%Y-%m-%d").to_string(),
            max_daily_downloads: 5,
        }
    }

    fn get_counter_key() -> Vec<u8> {
        let machine_id = match Self::get_machine_id() {
            Ok(id) => id,
            Err(_) => "DefaultCounterKey".to_string(),
        };

        let digest = digest::digest(&digest::SHA256, machine_id.as_bytes());
        digest.as_ref()[..16].to_vec()
    }

    fn get_machine_id() -> Result<String, AppError> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(id) = fs::read_to_string("/etc/machine-id") {
                return Ok(id.trim().to_string());
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("ioreg")
                .args(["-rd1", "-c", "IOPlatformExpertDevice"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().find(|line| line.contains("IOPlatformUUID")) {
                    if let Some(uuid_start) = line.find("\"") {
                        if let Some(uuid_end) = line[uuid_start + 1..].find("\"") {
                            return Ok(line[uuid_start + 1..uuid_start + 1 + uuid_end].to_string());
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE\\Microsoft\\Cryptography") {
                if let Ok(guid) = key.get_value::<String, _>("MachineGuid") {
                    return Ok(guid);
                }
            }
        }

        match hostname::get() {
            Ok(name) => Ok(name.to_string_lossy().to_string()),
            Err(_) => Err(AppError::General("Could not determine machine ID".to_string())),
        }
    }

    fn save_to_disk(&self) -> Result<(), AppError> {
        let counter_path = get_counter_path()?;
        
        let content = format!("{},{}", self.date, self.today_count);
        
        let key = hmac::Key::new(hmac::HMAC_SHA256, &Self::get_counter_key());
        let signature = hmac::sign(&key, content.as_bytes());
        let signature_b64 = general_purpose::STANDARD.encode(signature.as_ref());
        
        let data_with_signature = format!("{}\n{}", content, signature_b64);
        fs::write(counter_path, data_with_signature)?;
        
        Ok(())
    }

    fn load_from_disk() -> Result<Self, AppError> {
        let counter_path = get_counter_path()?;

        if !counter_path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(&counter_path)?;
        let parts: Vec<&str> = contents.split('\n').collect();
        
        if parts.len() != 2 {
            return Ok(Self::new());
        }
        
        let content = parts[0];
        let signature_b64 = parts[1];
        
        let key = hmac::Key::new(hmac::HMAC_SHA256, &Self::get_counter_key());
        match general_purpose::STANDARD.decode(signature_b64) {
            Ok(signature) => {
                match hmac::verify(&key, content.as_bytes(), &signature) {
                    Ok(_) => {
                        let data_parts: Vec<&str> = content.split(',').collect();
                        if data_parts.len() != 2 {
                            return Ok(Self::new());
                        }

                        let date = data_parts[0].to_string();
                        let today = Local::now().format("%Y-%m-%d").to_string();
                        
                        if date != today {
                            return Ok(Self::new());
                        }
                        
                        match data_parts[1].parse::<u32>() {
                            Ok(count) => Ok(Self {
                                today_count: count,
                                date,
                                max_daily_downloads: 5,
                            }),
                            Err(_) => Ok(Self::new()),
                        }
                    },
                    Err(_) => {
                        println!("{}", "Warning: Download counter validation failed. Counter has been reset.".yellow());
                        Ok(Self::new())
                    }
                }
            },
            Err(_) => Ok(Self::new()),
        }
    }

    fn increment(&mut self) -> Result<(), AppError> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        if today != self.date {
            self.date = today;
            self.today_count = 0;
        }

        self.today_count += 1;
        self.save_to_disk()?;

        Ok(())
    }

    fn can_download(&self) -> bool {
        let today = Local::now().format("%Y-%m-%d").to_string();
        if today != self.date {
            return true;
        }

        self.today_count < self.max_daily_downloads
    }

    fn remaining_downloads(&self) -> u32 {
        if self.today_count >= self.max_daily_downloads {
            0
        } else {
            self.max_daily_downloads - self.today_count
        }
    }
}

fn get_counter_path() -> Result<PathBuf, AppError> {
    let mut path = dirs::data_local_dir()
        .ok_or_else(|| AppError::PathError("Could not find local data directory".to_string()))?;

    path.push("rustloader");
    fs::create_dir_all(&path)?;

    path.push("download_counter.dat");
    Ok(path)
}

/// Check if there is an active network connection
async fn check_network_connectivity() -> bool {
    // Try to connect to multiple reliable hosts to check connectivity
    let hosts = [
        "1.1.1.1:53",        // Cloudflare DNS
        "8.8.8.8:53",        // Google DNS
        "208.67.222.222:53", // OpenDNS
    ];
    
    for host in &hosts {
        match tokio::net::TcpStream::connect(host).await {
            Ok(_) => {
                debug!("Network connectivity confirmed via {}", host);
                return true;
            },
            Err(e) => {
                debug!("Could not connect to {}: {}", host, e);
            }
        }
    }
    
    // All connection attempts failed
    false
}

/// Analyze error outputs to determine network error types
fn analyze_network_error(error: &io::Error, stderr_output: &str) -> (NetworkErrorKind, String, bool) {
    let kind = match error.kind() {
        io::ErrorKind::ConnectionReset | 
        io::ErrorKind::ConnectionAborted | 
        io::ErrorKind::BrokenPipe => NetworkErrorKind::ConnectionInterrupted,
        
        io::ErrorKind::TimedOut |
        io::ErrorKind::WouldBlock => NetworkErrorKind::Timeout,
        
        io::ErrorKind::NotFound => NetworkErrorKind::DnsResolutionFailure,
        
        io::ErrorKind::PermissionDenied => NetworkErrorKind::RateLimited,
        
        _ => NetworkErrorKind::Other,
    };
    
    // Analyze stderr to potentially refine the network error kind
    let (refined_kind, message, retriable) = if stderr_output.contains("429 Too Many Requests") {
        (NetworkErrorKind::RateLimited, 
         "The server is rate limiting your requests. Try again later.".to_string(),
         true)
    } else if stderr_output.contains("403 Forbidden") {
        (NetworkErrorKind::RateLimited, 
         "Access forbidden by the server. The content may be protected.".to_string(),
         false)
    } else if stderr_output.contains("404 Not Found") {
        (NetworkErrorKind::ContentUnavailable, 
         "The requested content is no longer available.".to_string(),
         false)
    } else if stderr_output.contains("DNS") && stderr_output.contains("resolution") {
        (NetworkErrorKind::DnsResolutionFailure, 
         "Could not resolve the domain name. Check your DNS settings or the URL.".to_string(),
         true)
    } else if stderr_output.contains("timeout") || stderr_output.contains("timed out") {
        (NetworkErrorKind::Timeout, 
         "The connection timed out. The server might be busy or your connection is slow.".to_string(),
         true)
    } else if stderr_output.contains("500") || stderr_output.contains("503") {
        (NetworkErrorKind::ServerError(503), 
         "The server is experiencing issues or is currently unavailable.".to_string(),
         true)
    } else if stderr_output.contains("connection") && 
              (stderr_output.contains("reset") || stderr_output.contains("closed")) {
        (NetworkErrorKind::ConnectionInterrupted, 
         "The connection was interrupted unexpectedly. Your network might be unstable.".to_string(),
         true)
    } else {
        // Default error analysis based on the kind
        let message = match kind {
            NetworkErrorKind::ConnectionInterrupted => 
                "The connection was interrupted. Your network may be unstable.".to_string(),
            NetworkErrorKind::Timeout => 
                "The connection timed out. The server might be busy or your connection is slow.".to_string(),
            NetworkErrorKind::DnsResolutionFailure => 
                "Could not resolve the host name. Check your internet connection and DNS settings.".to_string(),
            NetworkErrorKind::RateLimited => 
                "You may be downloading too quickly. The server is limiting your requests.".to_string(),
            NetworkErrorKind::ServerError(code) => 
                format!("Server error {} occurred. The server might be experiencing issues.", code),
            NetworkErrorKind::ConnectivityIssue => 
                "Network connectivity issue detected. Check your internet connection.".to_string(),
            NetworkErrorKind::ContentUnavailable => 
                "The requested content is no longer available.".to_string(),
            NetworkErrorKind::Other => 
                format!("Network error: {}", error),
        };
        
        // Most network errors are retriable except for content unavailability
        let retriable = kind != NetworkErrorKind::ContentUnavailable;
        
        (kind, message, retriable)
    };
    
    (refined_kind, message, retriable)
}

struct DownloadPromo {
    download_messages: Vec<String>,
    completion_messages: Vec<String>,
}

impl DownloadPromo {
    fn new() -> Self {
        Self {
            download_messages: vec![
                "🤖 AI-powered video enhancement available in Rustloader Pro! 🤖".to_string(),
                "💎 Get advanced subtitle features with Rustloader Pro! 💎".to_string(),
                "✨ Unlock advanced post-processing with Rustloader Pro! ✨".to_string(),
            ],
            completion_messages: vec![
                "✨ Enjoy your download! Upgrade to Pro for AI-powered features: rustloader.com/pro ✨".to_string(),
                "🚀 Rustloader Pro removes daily limits and adds AI capabilities. Learn more: rustloader.com/pro 🚀".to_string(),
                "💎 Thanks for using Rustloader! Upgrade to Pro for advanced features: rustloader.com/pro 💎".to_string(),
            ],
        }
    }

    fn get_random_download_message(&self) -> &str {
        let idx = rand::thread_rng().gen_range(0..self.download_messages.len());
        &self.download_messages[idx]
    }

    fn get_random_completion_message(&self) -> &str {
        let idx = rand::thread_rng().gen_range(0..self.completion_messages.len());
        &self.completion_messages[idx]
    }
}

struct YtdlpCommandBuilder {
    format: String,
    quality: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    url: String,
    output_path: String,
    use_playlist: bool,
    download_subtitles: bool,
    force_download: bool,
    bitrate: Option<String>,
}

impl YtdlpCommandBuilder {
    fn new(url: &str, output_path: &str) -> Self {
        Self {
            format: "mp4".to_string(),
            quality: None,
            start_time: None,
            end_time: None,
            url: url.to_string(),
            output_path: output_path.to_string(),
            use_playlist: false,
            download_subtitles: false,
            force_download: false,
            bitrate: None,
        }
    }
    
    fn with_format(mut self, format: &str) -> Self {
        self.format = format.to_string();
        self
    }
    
    fn with_quality(mut self, quality: Option<&str>) -> Self {
        self.quality = quality.map(|s| s.to_string());
        self
    }
    
    fn with_time_range(mut self, start_time: Option<&String>, end_time: Option<&String>) -> Self {
        self.start_time = start_time.cloned();
        self.end_time = end_time.cloned();
        self
    }
    
    fn with_playlist(mut self, use_playlist: bool) -> Self {
        self.use_playlist = use_playlist;
        self
    }
    
    fn with_subtitles(mut self, download_subtitles: bool) -> Self {
        self.download_subtitles = download_subtitles;
        self
    }
    
    fn with_force_download(mut self, force: bool) -> Self {
        self.force_download = force;
        self
    }
    
    fn with_bitrate(mut self, bitrate: Option<&String>) -> Self {
        self.bitrate = bitrate.cloned();
        self
    }
    
    fn build(self) -> AsyncCommand {
        let mut command = AsyncCommand::new("yt-dlp");
        
        let ffmpeg_required = self.format == "mp3" || 
                            self.start_time.is_some() || 
                            self.end_time.is_some();
        
        if ffmpeg_required && !*FFMPEG_AVAILABLE {
            if self.format == "mp3" {
                println!("{}", "⚠️ ERROR: FFmpeg is required for audio conversion but not found. ⚠️".bright_red());
                println!("{}", "The download will likely fail. Please install FFmpeg and try again.".bright_red());
            } else if self.start_time.is_some() || self.end_time.is_some() {
                println!("{}", "⚠️ ERROR: FFmpeg is required for time-based extraction but not found. ⚠️".bright_red());
                println!("{}", "The download will likely fail. Please install FFmpeg and try again.".bright_red());
            } else {
                println!("{}", "⚠️ Warning: FFmpeg not found. Some features may not work correctly. ⚠️".yellow());
            }
        }
        
        // Memory optimization for large files (>2GB)
        command.arg("--buffer-size").arg(format!("{}K", BUFFER_SIZE / 1024));
        
        // Limit the number of concurrent fragments to prevent memory bloat
        command.arg("--concurrent-fragments").arg("4");
        
        // Add file size limit check to avoid unexpected out-of-memory conditions
        command.arg("--max-filesize").arg("10G"); // Set reasonable 10GB limit 
        
        let aria2c_available = std::process::Command::new("aria2c")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        if aria2c_available {
            // Configure aria2c for better memory handling
            command.arg("--downloader").arg("aria2c");
            command.arg("--downloader-args").arg("aria2c:-x4"); // Max 4 connections
            command.arg("--downloader-args").arg(format!("aria2c:-k{}", BUFFER_SIZE / 1024)); // Use same buffer size 
            command.arg("--downloader-args").arg("aria2c:--file-allocation=none"); // Avoid preallocation
            command.arg("--downloader-args").arg("aria2c:--disk-cache=64M"); // Limit disk cache
        } else {
            command.arg("--downloader").arg("yt-dlp");
            // Limit memory usage for internal downloader
            command.arg("--limit-rate").arg("15M"); // Reasonable download rate limit to prevent memory spikes
        }
        
        if self.force_download {
            command.arg("--no-continue");
            command.arg("--no-part-file");
        }
        
        if self.format == "mp3" {
            command
                .arg("-f")
                .arg("bestaudio[ext=m4a]")
                .arg("--extract-audio")
                .arg("--audio-format")
                .arg("mp3");
    
            command.arg("--audio-quality").arg("7");
            command
                .arg("--postprocessor-args")
                .arg(format!("ffmpeg:-b:a {}", FREE_MP3_BITRATE));
    
            println!("{}", "⭐ Limited to 128kbps audio. Upgrade to Pro for studio-quality audio. ⭐".yellow());
        } else if let Some(quality_value) = &self.quality {
            println!("{}: {}", "Selected video quality".blue(), quality_value);
    
            let format_string = match quality_value.as_str() {
                "480" => "bestvideo[height<=480]+bestaudio/best[height<=480]/best",
                "720" => "bestvideo[height<=720]+bestaudio/best[height<=720]/best",
                "1080" => "bestvideo[height<=1080]+bestaudio/best[height<=1080]/best",
                "2160" => "bestvideo[height<=2160]+bestaudio/best[height<=2160]/best",
                _ => "best",
            };
    
            command.arg("-f").arg(format_string);
            command.arg("--verbose");
        }
        
        command.arg("-o").arg(&self.output_path);
        
        if self.use_playlist {
            command.arg("--yes-playlist");
            println!("{}", "Playlist mode enabled - will download all videos in playlist".yellow());
        } else {
            command.arg("--no-playlist");
        }
        
        if self.download_subtitles {
            command.arg("--write-subs").arg("--sub-langs").arg("all");
            println!("{}", "Subtitles will be downloaded if available".blue());
        }
        
        if self.start_time.is_some() || self.end_time.is_some() {
            let mut time_args = String::new();
    
            if let Some(start) = &self.start_time {
                time_args.push_str(&format!("-ss {} ", start));
            }
    
            if let Some(end) = &self.end_time {
                time_args.push_str(&format!("-to {} ", end));
            }
    
            if !time_args.is_empty() {
                command
                    .arg("--postprocessor-args")
                    .arg(format!("ffmpeg:{}", time_args.trim()));
            }
        }
        
        command.arg("--socket-timeout").arg("30");
        command.arg("--retries").arg("10");
        command.arg("--fragment-retries").arg("10");
        command.arg("--throttled-rate").arg("100K");
        command.arg("--newline");
        command
            .arg("--progress-template")
            .arg("download:%(progress.downloaded_bytes)s/%(progress.total_bytes)s");
        command.arg("--user-agent")
            .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");
        
        command.arg(self.url);
        
        command
    }
}

fn extract_video_id(url: &str) -> Option<String> {
    let is_valid_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';

    if let Some(v_pos) = url.find("v=") {
        let id_start = v_pos + 2;
        let id_end = url[id_start..]
            .find(|c: char| !is_valid_char(c))
            .map_or(url.len(), |pos| id_start + pos);

        let extracted = &url[id_start..id_end];

        if extracted.len() >= 8 && extracted.len() <= 12 && extracted.chars().all(is_valid_char) {
            return Some(extracted.to_string());
        }
    } else if url.contains("youtu.be/") {
        let parts: Vec<&str> = url.split("youtu.be/").collect();
        if parts.len() < 2 {
            return None;
        }

        let id_part = parts[1];
        let id_end = id_part
            .find(|c: char| !is_valid_char(c))
            .map_or(id_part.len(), |pos| pos);

        let extracted = &id_part[..id_end];

        if extracted.len() >= 8 && extracted.len() <= 12 && extracted.chars().all(is_valid_char) {
            return Some(extracted.to_string());
        }
    }

    None
}

fn sanitize_filename(filename: &str) -> Result<String, AppError> {
    let sanitized: String = filename
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    if sanitized.is_empty() || sanitized.len() < filename.len() / 2 {
        Err(AppError::ValidationError("Invalid filename after sanitization".to_string()))
    } else {
        Ok(sanitized)
    }
}

fn clear_partial_downloads(url: &str) -> Result<(), AppError> {
    println!("{}", "Clearing partial downloads to avoid resumption errors...".blue());

    let video_id = match extract_video_id(url) {
        Some(id) => sanitize_filename(&id)?,
        None => {
            println!("{}", "Could not extract video ID, skipping partial download cleanup.".yellow());
            return Ok(());
        }
    };

    if video_id.len() < 8 || video_id.len() > 12 {
        println!("{}", "Extracted video ID has suspicious length, skipping cleanup.".yellow());
        return Ok(());
    }

    let mut download_dirs = Vec::new();
    
    if let Some(mut home_path) = dirs::home_dir() {
        home_path.push("Downloads");
        home_path.push("rustloader");
        download_dirs.push(home_path.clone());

        let mut videos_path = home_path.clone();
        videos_path.push("videos");
        download_dirs.push(videos_path);

        let mut audio_path = home_path;
        audio_path.push("audio");
        download_dirs.push(audio_path);
    }

    download_dirs.push(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let mut total_removed = 0;
    for dir in download_dirs {
        if dir.exists() {
            match safe_cleanup(&dir, &video_id) {
                Ok(count) => {
                    if count > 0 {
                        println!("{} {} {}", "Removed".green(), count, format!("partial files from {:?}", dir).green());
                        total_removed += count;
                    }
                }
                Err(e) => {
                    println!("{}: {} {:?}", "Warning".yellow(), e, dir);
                }
            }
        }
    }

    if total_removed > 0 {
        println!("{} {}", "Total partial downloads removed:".green(), total_removed);
    } else {
        println!("{}", "No partial downloads found to clean up.".blue());
    }

    println!("{}", "Partial download cleanup completed.".green());
    Ok(())
}

fn safe_cleanup(dir: &PathBuf, video_id: &str) -> Result<usize, AppError> {
    if !crate::security::apply_rate_limit("file_cleanup", 3, std::time::Duration::from_secs(30)) {
        return Err(AppError::ValidationError("Too many file operations. Please try again later.".to_string()));
    }

    crate::security::validate_path_safety(dir)?;

    if !video_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::SecurityViolation);
    }

    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        // Use flatten to avoid the nested if let
        for entry in entries.flatten() {
            let path = entry.path();

            if let Err(e) = crate::security::validate_path_safety(&path) {
                println!("{}: {:?} - {}", "Skipping unsafe path".red(), path, e);
                continue;
            }

            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy();

                    if file_name_str.contains(video_id) && (file_name_str.ends_with(".part") || file_name_str.ends_with(".ytdl")) {
                        if file_name_str.chars().all(|c| {
                            c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' '
                        }) {
                            match std::fs::remove_file(&path) {
                                Ok(_) => {
                                    count += 1;
                                }
                                Err(e) => {
                                    println!("{}: {}", "Failed to remove file".red(), e);
                                }
                            }
                        } else {
                            println!("{}: {}", "Skipping file with suspicious characters".yellow(), file_name_str);
                        }
                    }
                }
            }
        }
    }

    Ok(count)
}

async fn get_video_title(url: &str) -> Result<String, AppError> {
    let mut command = AsyncCommand::new("yt-dlp");
    command
        .arg("--get-title")
        .arg("--no-playlist")
        .arg("--")
        .arg(url);

    let output = command.output().await.map_err(AppError::IoError)?;

    if !output.status.success() {
        return Err(AppError::DownloadError("Failed to get video title".to_string()));
    }

    let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if title.is_empty() {
        return Err(AppError::DownloadError("Could not determine video title".to_string()));
    }

    Ok(title)
}

fn check_if_video_exists(download_dir: &Path, format: &str, video_title: &str) -> Option<PathBuf> {
    let safe_title = regex::escape(video_title);
    let file_pattern = format!("{}.*\\.{}", safe_title, format);
    
    match Regex::new(&file_pattern) {
        Ok(re) => {
            if let Ok(entries) = fs::read_dir(download_dir) {
                for entry in entries.flatten() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if re.is_match(file_name) {
                            return Some(entry.path());
                        }
                    }
                }
            }
            None
        },
        Err(_) => None,
    }
}

fn prompt_for_redownload() -> Result<bool, AppError> {
    print!("This video has already been downloaded. Do you want to download it again? (y/n): ");
    io::stdout().flush().map_err(AppError::IoError)?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(AppError::IoError)?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

fn format_output_path_with_timestamp<P: AsRef<Path>>(download_dir: P, format: &str, timestamp: &str) -> Result<String, AppError> {
    validate_path_safety(download_dir.as_ref())?;

    match format {
        "mp3" | "mp4" | "webm" | "m4a" | "flac" | "wav" | "ogg" => {}
        _ => return Err(AppError::ValidationError(format!("Invalid output format: {}", format)))
    }

    let filename_template = format!("%(title)s_duplicate_{}.{}", timestamp, format);
    let path_buf = download_dir.as_ref().join(&filename_template);

    let path_str = path_buf
        .to_str()
        .ok_or_else(|| AppError::PathError("Invalid path encoding".to_string()))?
        .to_string();

    Ok(path_str)
}

#[allow(clippy::too_many_arguments)]
pub async fn download_video_free(
    url: &str,
    quality: Option<&str>,
    format: &str,
    start_time: Option<&String>,
    end_time: Option<&String>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<&String>,
    force_download: bool,
    bitrate: Option<&String>,
) -> Result<(), AppError> {
    validate_url(url)?;
    
    if let Some(start) = start_time {
        validate_time_format(start)?;
    }

    if let Some(end) = end_time {
        validate_time_format(end)?;
    }

    if let Some(rate) = bitrate {
        validate_bitrate(rate)?;
    }

    let mut counter = DownloadCounter::load_from_disk()?;
    if !force_download && !counter.can_download() {
        println!("{}", "⚠️ Daily download limit reached for free version ⚠️".bright_red());
        println!("{}", "🚀 Upgrade to Rustloader Pro for unlimited downloads: rustloader.com/pro 🚀".bright_yellow());
        return Err(AppError::DailyLimitExceeded);
    }

    println!("{} {}", "Downloads remaining today:".blue(), counter.remaining_downloads().to_string().green());
    println!("{}: {}", "Download URL".blue(), url);
    println!("{}", "Fetching video information...".blue());

    let folder_type = if format == "mp3" { "audio" } else { "videos" };
    let download_dir = initialize_download_dir(output_dir.map(|s| s.as_str()), "rustloader", folder_type)?;
    
    let mut should_use_unique_filename = false;
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();

    if !force_download && !use_playlist {
        match get_video_title(url).await {
            Ok(video_title) => {
                if let Some(existing_file) = check_if_video_exists(&download_dir, format, &video_title) {
                    println!("{}: {:?}", "Found existing download".yellow(), existing_file);

                    if !prompt_for_redownload()? {
                        println!("{}", "Download cancelled.".green());
                        return Ok(());
                    }

                    should_use_unique_filename = true;
                    println!("{}: Will append timestamp to filename", "Duplicate download".blue());
                }
            }
            Err(e) => {
                println!("{}: {}", "Warning: Could not get video title".yellow(), e);
                println!("{}", "Proceeding with download without duplicate check...".yellow());
            }
        }
    }

    if force_download {
        println!("{}", "Force download mode enabled - clearing partial downloads".blue());
        if let Err(e) = clear_partial_downloads(url) {
            println!("{}", format!("Warning: Could not clear partial downloads: {}. Continuing anyway.", e).yellow());
        }
    }

    let output_path = if should_use_unique_filename {
        format_output_path_with_timestamp(&download_dir, format, &timestamp)?
    } else {
        format_output_path(&download_dir, format)?
    };

    let progress = Arc::new(DownloadProgress::new());
    let pb = Arc::new(ProgressBar::new(100));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    pb.set_message(format!("Size: {} | Speed: {} | ETA: {}", "Calculating...", "Connecting...", "Calculating..."));

    let promo = DownloadPromo::new();
    println!("\n{}\n", promo.get_random_download_message().bright_yellow());

    println!("{}: {}", "Video quality".blue(), quality.unwrap_or("auto"));
    
    // Execute the download with retries
    let mut retry_count = 0;
    let mut stderr_output = String::new();
    let mut successful = false;
    
    'retry_loop: while retry_count <= MAX_RETRIES {
        if retry_count > 0 {
            // If we're retrying, first check network connectivity
            info!("Checking network connectivity before retry #{}", retry_count);
            pb.set_message(format!("Checking network connectivity... {}", progress.format_retry_status()));
            
            if !check_network_connectivity().await {
                warn!("No network connectivity detected. Waiting for network...");
                println!("{}", "Network connectivity issue detected. Waiting for connection...".yellow());
                
                // Wait for network to return before continuing
                let mut connectivity_check_attempts = 0;
                let max_connectivity_attempts = 5;
                
                while connectivity_check_attempts < max_connectivity_attempts {
                    pb.set_message(format!("Waiting for network... Attempt {}/{}", 
                        connectivity_check_attempts + 1, max_connectivity_attempts));
                    sleep(Duration::from_millis(NETWORK_CHECK_TIMEOUT_MS)).await;
                    
                    if check_network_connectivity().await {
                        info!("Network connectivity restored after {} attempts", connectivity_check_attempts + 1);
                        println!("{}", "Network connectivity restored. Resuming download...".green());
                        break;
                    }
                    
                    connectivity_check_attempts += 1;
                }
                
                if connectivity_check_attempts >= max_connectivity_attempts {
                    warn!("Failed to restore network connectivity after multiple attempts");
                    return Err(AppError::NetworkError {
                        kind: NetworkErrorKind::ConnectivityIssue,
                        message: "No network connectivity. Please check your internet connection and try again.".to_string(),
                        retriable: true,
                    });
                }
            }
            
            // Apply exponential backoff between retries
            let retry_delay = progress.get_retry_delay_ms();
            info!("Retrying download (attempt {}/{}). Waiting {}ms before retry", 
                retry_count, MAX_RETRIES, retry_delay);
            println!("{}: {}/{}", "Retrying download".yellow(), retry_count, MAX_RETRIES);
            pb.set_message(format!("Waiting before retry... {}", progress.format_retry_status()));
            sleep(Duration::from_millis(retry_delay)).await;
            
            // Prepare for retry
            progress.prepare_for_retry();
            pb.set_message("Reconnecting...");
            
            // If we had a connection interruption or stall, we might be able to resume
            if progress.is_resumable() {
                println!("{}", "Attempting to resume download from where it left off...".blue());
            } else {
                // For non-resumable errors, we need to rebuild command with force flag
                println!("{}", "Cannot resume download. Starting from beginning...".yellow());
            }
        }
        
        // Build a fresh command for each attempt
        let mut command = YtdlpCommandBuilder::new(url, &output_path)
            .with_format(format)
            .with_quality(quality)
            .with_time_range(start_time, end_time)
            .with_playlist(use_playlist)
            .with_subtitles(download_subtitles)
            .with_force_download(retry_count > 0 && !progress.is_resumable() || force_download)
            .with_bitrate(bitrate)
            .build();

        if retry_count == 0 {
            println!("{}", "Starting download...".green());
        } else {
            println!("{}", format!("Restarting download (attempt {}/{})...", retry_count, MAX_RETRIES).yellow());
        }

        // Spawn the download process
        let mut child = match command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(child) => child,
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => {
                        error!("yt-dlp executable not found");
                        eprintln!("{}", "Error: yt-dlp executable not found. Please ensure it's installed and in your PATH.".red());
                        return Err(AppError::MissingDependency("yt-dlp".to_string()));
                    }
                    io::ErrorKind::PermissionDenied => {
                        error!("Permission denied when running yt-dlp: {}", e);
                        eprintln!("{}", "Error: Permission denied when running yt-dlp. Check your file permissions.".red());
                        return Err(AppError::IoError(e));
                    }
                    _ => {
                        // For network errors, try to retry
                        let (kind, message, retriable) = analyze_network_error(&e, &stderr_output);
                        warn!("Failed to execute yt-dlp command: {} - {:?}", message, kind);
                        
                        if retriable && retry_count < MAX_RETRIES {
                            println!("{}: {}", "Network error".yellow(), message);
                            stderr_output.clear();
                            retry_count += 1;
                            progress.prepare_for_retry();
                            continue 'retry_loop;
                        } else {
                            error!("Fatal network error: {}", message);
                            eprintln!("{}", format!("Error: {}", message).red());
                            return Err(AppError::NetworkError { kind, message, retriable });
                        }
                    }
                }
            }
        };

        // Create a channel to collect stderr for later analysis
        let (stderr_tx, mut stderr_rx) = tokio::sync::mpsc::channel::<String>(100);
        
        // Set up stall detection with cancellation flag
        let progress_for_stall = Arc::clone(&progress);
        let pb_for_stall = Arc::clone(&pb);
        let stall_abort = Arc::new(AtomicBool::new(false));
        let stall_abort_clone = Arc::clone(&stall_abort);
        
        let stall_detection = tokio::spawn(async move {
            let mut stalled_counter = 0;
            loop {
                // Check if we should abort
                if stall_abort_clone.load(Ordering::SeqCst) {
                    return false;
                }
                
                sleep(Duration::from_secs(5)).await;
                
                // Check again after sleep
                if stall_abort_clone.load(Ordering::SeqCst) {
                    return false;
                }
                
                if progress_for_stall.is_stalled() {
                    stalled_counter += 1;
                    warn!("Download stalled detection: count={}", stalled_counter);
                    
                    if stalled_counter >= 3 {
                        warn!("Download stalled for too long, triggering retry");
                        pb_for_stall.set_message("Download stalled, preparing to retry...");
                        return true; // Signal stall detected
                    }
                    
                    pb_for_stall.set_message(format!("Download stalled! Waiting to resume... ({}s)", 
                        STALL_DETECTION_SECONDS * stalled_counter));
                } else {
                    stalled_counter = 0;
                }
            }
        });

        // Process stdout to track progress with memory optimization
        if let Some(stdout) = child.stdout.take() {
            // Use a properly sized buffer for optimal memory usage
            let stdout_buffered = BufReader::with_capacity(BUFFER_SIZE, stdout);
            let mut lines = stdout_buffered.lines();
            let pb_clone = Arc::clone(&pb);
            let progress_clone = Arc::clone(&progress);

            tokio::spawn(async move {
                // Preallocate a reasonable-sized string to avoid reallocations
                let mut line_buffer = String::with_capacity(256);
                
                // Track updates to limit processing frequency
                let mut last_gui_update = Instant::now();
                const GUI_UPDATE_INTERVAL_MS: u64 = 100; // Update UI every 100ms maximum
                
                while let Ok(Some(line)) = lines.next_line().await {
                    // Handle download progress updates
                    if line.starts_with("download:") {
                        if let Some(progress_str) = line.strip_prefix("download:") {
                            // Process the download progress
                            let now = Instant::now();
                            let should_update_ui = now.duration_since(last_gui_update).as_millis() > 
                                                  GUI_UPDATE_INTERVAL_MS as u128;
                            
                            // Parse progress info
                            let parts: Vec<&str> = progress_str.split('/').collect();
                            if parts.len() == 2 {
                                if let (Ok(downloaded), Ok(total)) = (
                                    parts[0].trim().parse::<u64>(),
                                    parts[1].trim().parse::<u64>(),
                                ) {
                                    if total > 0 {
                                        // Always update internal progress tracking
                                        progress_clone.update(downloaded, total);
                                        
                                        // But only update UI at specified intervals to reduce CPU/memory usage
                                        if should_update_ui {
                                            let percentage = progress_clone.get_percentage();
                                            pb_clone.set_position(percentage);
                                            
                                            // Format message only when updating UI
                                            line_buffer.clear();
                                            let size = progress_clone.format_file_size();
                                            let speed = progress_clone.format_speed();
                                            let eta = progress_clone.format_eta();
                                            
                                            // Use string concatenation to avoid allocations
                                            line_buffer.push_str("Size: ");
                                            line_buffer.push_str(&size);
                                            line_buffer.push_str(" | Speed: ");
                                            line_buffer.push_str(&speed);
                                            line_buffer.push_str(" | ETA: ");
                                            line_buffer.push_str(&eta);
                                            
                                            pb_clone.set_message(line_buffer.clone());
                                            last_gui_update = now;
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Only print non-progress messages
                        println!("{}", line);
                    }
                }
                
                // Explicitly drop large buffers
                drop(lines);
                line_buffer.clear();
                line_buffer.shrink_to_fit();
            });
        }

        // Process stderr to capture errors and judge if download is resumable
        if let Some(stderr) = child.stderr.take() {
            // Use optimized buffer size for stderr too
            let stderr_reader = BufReader::with_capacity(BUFFER_SIZE, stderr);
            let mut lines = stderr_reader.lines();
            let stderr_tx_clone = stderr_tx.clone();
            let progress_clone = Arc::clone(&progress);

            tokio::spawn(async move {
                // Preallocate a reasonable buffer size for stderr output
                let mut error_buffer = String::with_capacity(512);
                
                while let Ok(Some(line)) = lines.next_line().await {
                    // Only store important error messages for analysis
                    // This reduces memory usage for long-running downloads with many warnings
                    let is_important_error = line.contains("Error") || 
                                            line.contains("Failed") || 
                                            line.contains("HTTP") ||
                                            line.contains("Connection") ||
                                            line.contains("Resuming") ||
                                            line.contains("Unable to download");
                                            
                    if is_important_error {
                        // Save important stderr output for later analysis
                        // We use a limited capacity channel to prevent memory growth
                        let _ = stderr_tx_clone.send(line.clone()).await;
                    }
                    
                    // Determine if error suggests resumable download
                    if line.contains("HTTP Error 416") || line.contains("Requested Range Not Satisfiable") {
                        progress_clone.set_resumable(false);
                        eprintln!("{}", "Error: File already exists or download was previously completed.".red());
                    } else if line.contains("Resuming") {
                        progress_clone.set_resumable(true);
                        println!("{}", "Resuming download from previous state...".green());
                    } else if line.contains("Unable to download") || line.contains("403 Forbidden") || 
                            line.contains("404 Not Found") {
                        progress_clone.set_resumable(false);
                    }
                    
                    // Print the error to the console
                    eprintln!("{}", line.red());
                }
                
                // Explicitly drop large buffers
                drop(lines);
                error_buffer.clear();
                error_buffer.shrink_to_fit();
            });
        }

        // Spawn a task to collect stderr lines with memory limitations
        let stderr_collector = tokio::spawn(async move {
            // Preallocate a reasonably sized buffer for stderr collection
            let mut collected = String::with_capacity(4096); // 4KB initial capacity
            let mut line_count = 0;
            const MAX_ERROR_LINES: usize = 200; // Limit number of error lines collected
            
            while let Some(line) = stderr_rx.recv().await {
                // If we've collected too many lines, start dropping older ones
                // to prevent unbounded memory growth
                if line_count > MAX_ERROR_LINES && collected.len() > 2048 {
                    // Keep only the last half of the content when we reach the limit
                    let content_len = collected.len();
                    if let Some(pos) = collected[content_len/2..].find('\n') {
                        let truncate_pos = content_len/2 + pos + 1;
                        collected.replace_range(0..truncate_pos, "");
                        // Add truncation marker
                        collected.insert_str(0, "[...truncated...]\n");
                    }
                }
                
                // Add the new line
                collected.push_str(&line);
                collected.push('\n');
                line_count += 1;
                
                // If the buffer gets too large, force a capacity reduction
                if collected.capacity() > 16384 && collected.len() < collected.capacity() / 2 {
                    let temp = collected.clone();
                    collected = temp;
                    collected.shrink_to_fit();
                }
            }
            
            collected
        });

        // Wait for either child process to complete or stall detection to trigger
        let (status_result, is_stalled) = tokio::select! {
            status = child.wait() => (status, false),
            stalled = stall_detection => {
                if stalled.unwrap_or(false) {
                    // Kill the process if stalled
                    let _ = child.kill().await;
                    (Err(io::Error::new(io::ErrorKind::TimedOut, "Download stalled")), true)
                } else {
                    (Err(io::Error::new(io::ErrorKind::Other, "Stall detector exited unexpectedly")), false)
                }
            }
        };

        // Signal the stall detector to stop
        stall_abort.store(true, Ordering::SeqCst);
        
        // Get collected stderr output
        stderr_output = stderr_collector.await.unwrap_or_default();
        
        match status_result {
            Ok(status) => {
                if status.success() {
                    info!("Download completed successfully");
                    pb.finish_with_message("Download completed");
                    successful = true;
                    break 'retry_loop;
                } else {
                    let exit_code = status.code().unwrap_or(0);
                    warn!("Download failed with exit code {}", exit_code);
                    
                    // Check for specific non-retriable failures
                    if exit_code == 1 && format == "mp3" && !*FFMPEG_AVAILABLE {
                        error!("Download failed due to missing ffmpeg");
                        return Err(AppError::DownloadError(
                            "Download failed, likely due to missing or incompatible ffmpeg. Please install ffmpeg and try again.".to_string(),
                        ));
                    } else if start_time.is_some() || end_time.is_some() && !*FFMPEG_AVAILABLE {
                        error!("Time-based download failed due to missing ffmpeg");
                        return Err(AppError::DownloadError(
                            "Download with time selection failed. This feature requires a working ffmpeg installation.".to_string(),
                        ));
                    } else if retry_count < MAX_RETRIES {
                        // Analyze the error and determine if we should retry
                        if stderr_output.contains("429 Too Many Requests") || 
                           stderr_output.contains("rate limit") {
                            progress.set_resumable(true);
                            println!("{}", "Rate limit hit. Adding longer delay before retry...".yellow());
                        } else if stderr_output.contains("Connection") && 
                                (stderr_output.contains("reset") || 
                                 stderr_output.contains("closed") ||
                                 stderr_output.contains("timeout")) {
                            progress.set_resumable(true);
                            println!("{}", "Connection interrupted. Will attempt to resume...".yellow());
                        }
                        
                        retry_count += 1;
                        continue 'retry_loop;
                    } else {
                        // We've exhausted our retries
                        error!("Download failed after max retries");
                        return Err(AppError::DownloadError(
                            format!("yt-dlp command failed with exit code {} after {} retries. Please verify the URL and options provided.", 
                                exit_code, MAX_RETRIES)
                        ));
                    }
                }
            },
            Err(e) => {
                // Handle network errors
                if is_stalled {
                    warn!("Download stalled, preparing for retry");
                    if retry_count < MAX_RETRIES {
                        println!("{}", "Download stalled or timed out. Preparing to retry...".yellow());
                        progress.set_resumable(true);
                        retry_count += 1;
                        continue 'retry_loop;
                    } else {
                        error!("Download stalled too many times, giving up");
                        return Err(AppError::NetworkError {
                            kind: NetworkErrorKind::Timeout,
                            message: "Download stalled too many times. The server might be throttling connections or your network is unstable.".to_string(),
                            retriable: true,
                        });
                    }
                }
                
                // Analyze the error to determine what kind of network issue it is
                let (kind, message, retriable) = analyze_network_error(&e, &stderr_output);
                warn!("Download process error: {} - {:?}", message, kind);
                
                if retriable && retry_count < MAX_RETRIES {
                    println!("{}: {}", "Network error".yellow(), message);
                    retry_count += 1;
                    continue 'retry_loop;
                } else {
                    error!("Fatal network error: {}", message);
                    eprintln!("{}", format!("Error: {}", message).red());
                    return Err(AppError::NetworkError { kind, message, retriable });
                }
            }
        }
    }

    if !successful {
        // Should not reach here normally as we either succeed or return error in the loop
        error!("Download failed after {} retries", retry_count);
        return Err(AppError::DownloadError("Download failed after maximum retries".to_string()));
    }

    // Only increment counter if no retries were needed or the final retry succeeded
    if !force_download {
        info!("Incrementing download counter");
        counter.increment()?;
    }

    let _ = Notification::new()
        .summary("Download Complete")
        .body(&format!("{} file downloaded successfully.", format.to_uppercase()))
        .show();

    println!("{} {} {}", "Download completed successfully.".green(), format.to_uppercase(), "file saved.".green());
    println!("\n{}\n", promo.get_random_completion_message().bright_yellow());

    Ok(())
}