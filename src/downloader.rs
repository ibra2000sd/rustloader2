// src/downloader.rs

use crate::error::AppError;
use crate::utils::{format_output_path, initialize_download_dir, validate_bitrate, validate_time_format, validate_url, validate_path_safety};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use notify_rust::Notification;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as AsyncCommand;
use rand::Rng;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Local;
use dirs_next as dirs;
use ring::{digest, hmac};
use base64::{Engine as _, engine::general_purpose};
use hostname;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, Duration};
use std::sync::Mutex;
use humansize::{format_size, BINARY};
use std::io::{self, Write};
use regex::Regex;
use crate::dependency_validator::is_ffmpeg_available;

// Constants for free version limitations - audio bitrate limit
// Note: MAX_FREE_QUALITY removed as it's no longer used
const FREE_MP3_BITRATE: &str = "128K";

// Enhanced progress tracking
struct DownloadProgress {
    #[allow(dead_code)]
    start_time: Instant,
    last_update: Mutex<Instant>,
    downloaded_bytes: AtomicU64,
    total_bytes: AtomicU64,
    download_speed: Mutex<f64>,  // bytes per second
    last_speed_samples: Mutex<Vec<f64>>,
}

impl DownloadProgress {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_update: Mutex::new(Instant::now()),
            downloaded_bytes: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            download_speed: Mutex::new(0.0),
            last_speed_samples: Mutex::new(vec![]),
        }
    }
    
    fn update(&self, downloaded: u64, total: u64) {
        // Update bytes counters
        self.downloaded_bytes.store(downloaded, Ordering::SeqCst);
        self.total_bytes.store(total, Ordering::SeqCst);
        
        // Calculate and update speed
        let now = Instant::now();
        
        let mut last_update = self.last_update.lock().unwrap();
        let time_diff = now.duration_since(*last_update).as_millis();
        
        // Only update speed calculation every 100ms to avoid jitter
        if time_diff >= 100 {
            let mut last_speed_samples = self.last_speed_samples.lock().unwrap();
            let mut speed = self.download_speed.lock().unwrap();
            
            // Calculate current speed
            if let Some(last_downloaded) = self.downloaded_bytes.load(Ordering::SeqCst).checked_sub(downloaded) {
                let current_speed = last_downloaded as f64 / (time_diff as f64 / 1000.0);
                
                // Add to speed samples
                last_speed_samples.push(current_speed);
                if last_speed_samples.len() > 10 {
                    last_speed_samples.remove(0);
                }
                
                // Calculate average speed from samples
                let sum: f64 = last_speed_samples.iter().sum();
                *speed = sum / last_speed_samples.len() as f64;
            }
            
            *last_update = now;
        }
    }
    
    fn get_percentage(&self) -> u64 {
        let downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);
        
        if total == 0 {
            return 0;
        }
        
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
            },
            None => "Calculating...".to_string()
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
        
        format!("{} / {}", 
            format_size(downloaded, BINARY),
            format_size(total, BINARY)
        )
    }
}

// Updated promotional messages to focus on AI features instead of speed
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

// Download counter for tracking daily limits with secure storage
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
            max_daily_downloads: 5, // Free tier limit
        }
    }
    
    // Generate a unique key for counter encryption based on machine ID
    fn get_counter_key() -> Vec<u8> {
        // Try to get a machine-specific identifier
        let machine_id = match Self::get_machine_id() {
            Ok(id) => id,
            Err(_) => "DefaultCounterKey".to_string(), // Fallback if machine ID can't be determined
        };
        
        // Use SHA-256 to create a fixed-length key from the machine ID
        let digest = digest::digest(&digest::SHA256, machine_id.as_bytes());
        digest.as_ref().to_vec()
    }
    
    // Rewritten get_machine_id to avoid unreachable code warnings
    fn get_machine_id() -> Result<String, AppError> {
        #[cfg(target_os = "linux")]
        {
            // On Linux, try to use the machine-id
            match fs::read_to_string("/etc/machine-id") {
                Ok(id) => return Ok(id.trim().to_string()),
                Err(_) => {}  // Fall through to hostname fallback
            }
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, use the IOPlatformUUID
            use std::process::Command;
            
            let output_result = Command::new("ioreg")
                .args(["-rd1", "-c", "IOPlatformExpertDevice"])
                .output();
                
            if let Ok(output) = output_result {
                let stdout = String::from_utf8_lossy(&output.stdout);
                
                // Extract the UUID using a simple search
                if let Some(line) = stdout.lines().find(|line| line.contains("IOPlatformUUID")) {
                    if let Some(uuid_start) = line.find("\"") {
                        if let Some(uuid_end) = line[uuid_start + 1..].find("\"") {
                            return Ok(line[uuid_start + 1..uuid_start + 1 + uuid_end].to_string());
                        }
                    }
                }
            }
            // Fall through to hostname fallback
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, try to use the MachineGuid from registry
            use winreg::enums::*;
            use winreg::RegKey;
            
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(key) = hklm.open_subkey("SOFTWARE\\Microsoft\\Cryptography") {
                if let Ok(guid) = key.get_value::<String, _>("MachineGuid") {
                    return Ok(guid);
                }
            }
            // Fall through to hostname fallback
        }
        
        // Fallback for all platforms - use hostname
        match hostname::get() {
            Ok(name) => Ok(name.to_string_lossy().to_string()),
            Err(_) => Err(AppError::General("Could not determine machine ID".to_string())),
        }
    }
    
    // Encrypt counter data with HMAC signature
    fn encrypt_counter_data(&self) -> Result<String, AppError> {
        // Create the data string
        let content = format!("{},{}", self.date, self.today_count);
        
        // Create HMAC signature
        let key = hmac::Key::new(hmac::HMAC_SHA256, &Self::get_counter_key());
        let signature = hmac::sign(&key, content.as_bytes());
        let signature_b64 = general_purpose::STANDARD.encode(signature.as_ref());
        
        // Combine data and signature
        let full_data = format!("{}\n{}", content, signature_b64);
        
        // Base64 encode the full data
        Ok(general_purpose::STANDARD.encode(full_data.as_bytes()))
    }
    
    // Decrypt and verify counter data
    fn decrypt_counter_data(encrypted_data: &str) -> Result<(String, u32), AppError> {
        // Base64 decode the data
        let decoded_bytes = match general_purpose::STANDARD.decode(encrypted_data) {
            Ok(bytes) => bytes,
            Err(_) => return Err(AppError::SecurityViolation),
        };
        
        // Convert to string and split by newline
        let full_data = match String::from_utf8(decoded_bytes) {
            Ok(data) => data,
            Err(_) => return Err(AppError::SecurityViolation),
        };
        
        let parts: Vec<&str> = full_data.split('\n').collect();
        if parts.len() != 2 {
            return Err(AppError::SecurityViolation);
        }
        
        let content = parts[0];
        let signature_b64 = parts[1];
        
        // Verify signature
        let key = hmac::Key::new(hmac::HMAC_SHA256, &Self::get_counter_key());
        let signature_bytes = match general_purpose::STANDARD.decode(signature_b64) {
            Ok(bytes) => bytes,
            Err(_) => return Err(AppError::SecurityViolation),
        };
        
        match hmac::verify(&key, content.as_bytes(), &signature_bytes) {
            Ok(_) => {
                // Signature verified, parse the data
                let data_parts: Vec<&str> = content.split(',').collect();
                if data_parts.len() != 2 {
                    return Err(AppError::SecurityViolation);
                }
                
                let date = data_parts[0].to_string();
                let count: u32 = match data_parts[1].parse() {
                    Ok(c) => c,
                    Err(_) => return Err(AppError::SecurityViolation),
                };
                
                Ok((date, count))
            },
            Err(_) => Err(AppError::SecurityViolation),
        }
    }
    
    fn load_from_disk() -> Result<Self, AppError> {
        let counter_path = get_counter_path()?;
        
        if counter_path.exists() {
            let encrypted_contents = fs::read_to_string(&counter_path)?;
            
            match Self::decrypt_counter_data(&encrypted_contents) {
                Ok((date, count)) => {
                    // Check if date has changed
                    let today = Local::now().format("%Y-%m-%d").to_string();
                    if date != today {
                        return Ok(Self::new()); // Reset counter for new day
                    }
                    
                    Ok(Self {
                        today_count: count,
                        date,
                        max_daily_downloads: 5,
                    })
                },
                Err(_) => {
                    // If decryption fails, assume tampering and create a new counter
                    // with max downloads already used (as a penalty for tampering)
                    println!("{}", "Warning: Download counter validation failed. Counter has been reset.".yellow());
                    let mut counter = Self::new();
                    counter.today_count = counter.max_daily_downloads; // Use up all downloads as penalty
                    
                    // Save the new counter immediately
                    if let Err(e) = counter.save_to_disk() {
                        println!("{}: {}", "Error saving counter".red(), e);
                    }
                    
                    Ok(counter)
                }
            }
        } else {
            Ok(Self::new())
        }
    }
    
    fn save_to_disk(&self) -> Result<(), AppError> {
        let counter_path = get_counter_path()?;
        
        // Encrypt the counter data
        let encrypted_data = self.encrypt_counter_data()?;
        
        // Write to disk
        fs::write(counter_path, encrypted_data)?;
        Ok(())
    }
    
    fn increment(&mut self) -> Result<(), AppError> {
        // Check if date has changed
        let today = Local::now().format("%Y-%m-%d").to_string();
        if today != self.date {
            self.date = today;
            self.today_count = 0;
            println!("{}", "Daily download counter reset for new day.".blue());
        }
        
        self.today_count += 1;
        self.save_to_disk()?;
        
        Ok(())
    }
    
    fn can_download(&self) -> bool {
        // Check if date has changed
        let today = Local::now().format("%Y-%m-%d").to_string();
        if today != self.date {
            return true; // New day, reset counter
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

// New function to enable parallel downloads (replaces ensure_single_threaded_download)
fn enable_parallel_download(command: &mut AsyncCommand) {
    // Enable parallel downloading features
    command.arg("--concurrent-fragments").arg("4");  // Allow multiple fragments to download at once
    
    // Detect if aria2c is available for better parallel downloading
    let aria2c_available = std::process::Command::new("aria2c")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    if aria2c_available {
        // The issue is with the way arguments are passed to aria2c
        // Use correct format for aria2c options
        command.arg("--downloader").arg("aria2c");
        
        // Fix: Use separate parameters instead of single string with quotes
        // Original problematic line: command.arg("--downloader-args").arg("aria2c:'-x 4 -s 4 -k 1M'");
        command.arg("--downloader-args").arg("aria2c:-x4");  // 4 connections per server
        command.arg("--downloader-args").arg("aria2c:-k1M"); // 1MB per chunk
    } else {
        // Otherwise use built-in parallel download capability
        command.arg("--downloader").arg("yt-dlp");
    }
}

// Displays a promotional message during download
fn display_download_promo() {
    let promo = DownloadPromo::new();
    println!("{}", promo.get_random_download_message().bright_yellow());
}

// Displays a promotional message after download
fn display_completion_promo() {
    let promo = DownloadPromo::new();
    println!("\n{}\n", promo.get_random_completion_message().bright_yellow());
}

/// Extract YouTube video ID from URL with enhanced security
fn extract_video_id(url: &str) -> Option<String> {
    // Define strict character allowlist for video IDs
    let is_valid_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    
    // Extract video ID from YouTube URL patterns
    if let Some(v_pos) = url.find("v=") {
        let id_start = v_pos + 2;
        let id_end = url[id_start..]
            .find(|c: char| !is_valid_char(c))
            .map_or(url.len(), |pos| id_start + pos);
        
        let extracted = &url[id_start..id_end];
        
        // Additional validation - YouTube IDs are typically 11 characters
        if extracted.len() >= 8 && extracted.len() <= 12 && 
           extracted.chars().all(is_valid_char) {
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
        
        // Additional validation - YouTube IDs are typically 11 characters
        if extracted.len() >= 8 && extracted.len() <= 12 && 
           extracted.chars().all(is_valid_char) {
            return Some(extracted.to_string());
        }
    }
    
    None
}

/// Sanitize a filename to prevent command injection - improved whitelist approach
fn sanitize_filename(filename: &str) -> Result<String, AppError> {
    // Strict whitelist for allowed characters
    let sanitized: String = filename.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    
    // Enforce minimum length and validate that all characters passed the filter
    if sanitized.is_empty() || sanitized.len() < filename.len() / 2 {
        Err(AppError::ValidationError("Invalid filename after sanitization".to_string()))
    } else {
        Ok(sanitized)
    }
}

/// Clears any partial downloads with enhanced security and thorough cleanup
fn clear_partial_downloads(url: &str) -> Result<(), AppError> {
    println!("{}", "Clearing partial downloads to avoid resumption errors...".blue());
    
    // Get the video ID with enhanced extraction and validation
    let video_id = match extract_video_id(url) {
        Some(id) => {
            // Apply additional sanitization for extra security
            sanitize_filename(&id)?
        },
        None => {
            println!("{}", "Could not extract video ID, skipping partial download cleanup.".yellow());
            return Ok(());
        }
    };
    
    // Additional validation - ensure ID has reasonable length
    if video_id.len() < 8 || video_id.len() > 12 {
        println!("{}", "Extracted video ID has suspicious length, skipping cleanup.".yellow());
        return Ok(());
    }
    
    // Get multiple potential download directories as PathBufs
    let mut download_dirs = Vec::new();
    
    // Check standard download locations
    if let Some(mut home_path) = dirs::home_dir() {
        // Main download directory
        home_path.push("Downloads");
        home_path.push("rustloader");
        download_dirs.push(home_path.clone());
        
        // Check videos subdirectory
        let mut videos_path = home_path.clone();
        videos_path.push("videos");
        download_dirs.push(videos_path);
        
        // Check audio subdirectory
        let mut audio_path = home_path;
        audio_path.push("audio");
        download_dirs.push(audio_path);
    }
    
    // Add temporary directories that might contain partial downloads
    if let Some(mut temp_path) = dirs::cache_dir() {
        temp_path.push("rustloader");
        download_dirs.push(temp_path);
    }
    
    // Also check current directory
    download_dirs.push(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    
    println!("{} {}", "Looking for partial downloads with ID:".blue(), video_id);
    
    // Track total removed files
    let mut total_removed = 0;
    
    // Check all potential directories
    for dir in download_dirs {
        if dir.exists() {
            match safe_cleanup(&dir, &video_id) {
                Ok(count) => {
                    if count > 0 {
                        println!("{} {} {}", "Removed".green(), count, format!("partial files from {:?}", dir).green());
                        total_removed += count;
                    }
                },
                Err(e) => {
                    println!("{}: {} {:?}", "Warning".yellow(), e, dir);
                    // Continue with other directories even if one fails
                }
            }
        }
    }
    
    // Also attempt to find and remove any temp files with similar patterns
    let temp_dir = std::env::temp_dir();
    if temp_dir.exists() {
        match safe_cleanup(&temp_dir, &video_id) {
            Ok(count) => {
                if count > 0 {
                    println!("{} {} {}", "Removed".green(), count, format!("temp files from {:?}", temp_dir).green());
                    total_removed += count;
                }
            },
            Err(e) => {
                println!("{}: {} {:?}", "Warning".yellow(), e, temp_dir);
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

/// Unified safe cleanup implementation with enhanced security
fn safe_cleanup(dir: &PathBuf, video_id: &str) -> Result<usize, AppError> {
    let mut count = 0;
    
    // Apply rate limiting to file operations
    if !crate::security::apply_rate_limit("file_cleanup", 3, std::time::Duration::from_secs(30)) {
        return Err(AppError::ValidationError("Too many file operations. Please try again later.".to_string()));
    }
    
    // First validate the directory for security
    crate::security::validate_path_safety(dir)?;
    
    // Verify that video_id only contains safe characters again
    if !video_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::SecurityViolation);
    }
    
    // Process .part and .ytdl files only
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry_result in entries {
            if let Ok(entry) = entry_result {
                let path = entry.path();
                
                // Validate the file path is safe
                if let Err(e) = crate::security::validate_path_safety(&path) {
                    println!("{}: {:?} - {}", "Skipping unsafe path".red(), path, e);
                    continue;
                }
                
                if path.is_file() {
                    if let Some(file_name) = path.file_name() {
                        let file_name_str = file_name.to_string_lossy();
                        
                        // Check if this is a partial download matching our video ID
                        // Only remove files ending with .part or .ytdl
                        if file_name_str.contains(video_id) && 
                           (file_name_str.ends_with(".part") || file_name_str.ends_with(".ytdl")) {
                            // Double-check the file name for security
                            if file_name_str.chars().all(|c| 
                                c.is_ascii_alphanumeric() || 
                                c == '-' || c == '_' || c == '.' || c == ' '
                            ) {
                                // Get file metadata before removal (log for auditing)
                                if let Ok(metadata) = std::fs::metadata(&path) {
                                    println!("{} {} ({})", "Removing:".yellow(), file_name_str, 
                                             humansize::format_size(metadata.len(), humansize::BINARY));
                                } else {
                                    println!("{} {}", "Removing:".yellow(), file_name_str);
                                }
                                
                                // Remove the file safely
                                match std::fs::remove_file(&path) {
                                    Ok(_) => {
                                        count += 1;
                                        
                                        // Verify file was removed (double-check)
                                        if path.exists() {
                                            println!("{}: {} still exists", "Warning".red(), file_name_str);
                                        }
                                    },
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
    }
    
    // Log the operation
    println!("{} {} {}", "Cleaned up".green(), count, "partial download files".green());
    
    Ok(count)
}

// Updated to no longer limit video quality in free version
fn limit_video_quality(requested_quality: &str) -> &str {
    // Return the requested quality without restrictions
    requested_quality
}

// Modify audio command to limit quality to 128kbps
fn modify_audio_command(command: &mut AsyncCommand, bitrate: Option<&String>) {
    // If bitrate is specified in Pro version, it would be respected
    // but in free version, we enforce the limitation
    if let Some(rate) = bitrate {
        println!("{} {} {}", 
            "Requested audio bitrate:".yellow(), 
            rate, 
            "(Limited to 128K in free version)".yellow()
        );
    }

    // Add audio bitrate limitation for free version
    command.arg("--audio-quality").arg("7"); // 128kbps in yt-dlp scale (0-10)
    command.arg("--postprocessor-args").arg(format!("ffmpeg:-b:a {}", FREE_MP3_BITRATE));

    println!("{}", "⭐ Limited to 128kbps audio. Upgrade to Pro for studio-quality audio. ⭐".yellow());
}

// Function to extract video title from URL using yt-dlp
async fn get_video_title(url: &str) -> Result<String, AppError> {
    let mut command = AsyncCommand::new("yt-dlp");
    command.arg("--get-title")
           .arg("--no-playlist")
           .arg("--")
           .arg(url);
           
    let output = command.output().await
        .map_err(|e| AppError::IoError(e))?;
        
    if !output.status.success() {
        return Err(AppError::DownloadError("Failed to get video title".to_string()));
    }
    
    let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if title.is_empty() {
        return Err(AppError::DownloadError("Could not determine video title".to_string()));
    }
    
    Ok(title)
}

// Function to check if a video already exists
fn check_if_video_exists(download_dir: &Path, format: &str, video_title: &str) -> Option<PathBuf> {
    // Create a regex to match the video title pattern that yt-dlp would create
    // This is a simplified approach - we'd need to know exactly how yt-dlp formats filenames
    let safe_title = regex::escape(video_title);
    let file_pattern = format!("{}.*\\.{}", safe_title, format);
    let re = match Regex::new(&file_pattern) {
        Ok(re) => re,
        Err(_) => return None,
    };
    
    // Check if any files in the directory match this pattern
    if let Ok(entries) = fs::read_dir(download_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(file_name) = entry.file_name().to_str() {
                    if re.is_match(file_name) {
                        return Some(entry.path());
                    }
                }
            }
        }
    }
    
    None
}

// Function to prompt user for redownload confirmation
fn prompt_for_redownload() -> Result<bool, AppError> {
    print!("This video has already been downloaded. Do you want to download it again? (y/n): ");
    io::stdout().flush().map_err(|e| AppError::IoError(e))?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| AppError::IoError(e))?;
    
    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

// Helper function to create output path with timestamp
fn format_output_path_with_timestamp<P: AsRef<Path>>(
    download_dir: P, 
    format: &str,
    timestamp: &str
) -> Result<String, AppError> {
    // Validate path safety
    validate_path_safety(download_dir.as_ref())?;
    
    // Make sure the format is valid
    match format {
        "mp3" | "mp4" | "webm" | "m4a" | "flac" | "wav" | "ogg" => {},
        _ => return Err(AppError::ValidationError(format!("Invalid output format: {}", format))),
    }
    
    // Create a customized filename template with timestamp
    let filename_template = format!("%(title)s_duplicate_{}.{}", timestamp, format);
    
    // Use PathBuf for proper platform-specific path handling
    let path_buf = download_dir.as_ref().join(&filename_template);
    
    let path_str = path_buf
        .to_str()
        .ok_or_else(|| AppError::PathError("Invalid path encoding".to_string()))?
        .to_string();
    
    // Just return the path string directly
    Ok(path_str)
}

/// Download a video or audio file from the specified URL
/// Updated to remove quality restrictions and handle duplicate downloads
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
    // Validate URL more strictly
    validate_url(url)?;

    // Check daily download limit with secured counter
    let mut counter = DownloadCounter::load_from_disk()?;
    if !force_download && !counter.can_download() {
        println!("{}", "⚠️ Daily download limit reached for free version ⚠️".bright_red());
        println!("{}", "🚀 Upgrade to Rustloader Pro for unlimited downloads: rustloader.com/pro 🚀".bright_yellow());
        return Err(AppError::DailyLimitExceeded);
    }

    // Show remaining downloads
    println!("{} {}", 
        "Downloads remaining today:".blue(), 
        counter.remaining_downloads().to_string().green()
    );

    println!("{}: {}", "Download URL".blue(), url);

    // Get video title to check for existing file
    println!("{}", "Fetching video information...".blue());
    
    let mut should_use_unique_filename = false;
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    
    // Initialize download directory with enhanced security
    let folder_type = if format == "mp3" { "audio" } else { "videos" };
    let download_dir = initialize_download_dir(
        output_dir.map(|s| s.as_str()), 
        "rustloader", 
        folder_type
    )?;
    
    // Check for already downloaded file if not using force_download
    if !force_download {
        // Only attempt this check for regular videos, not playlists
        if !use_playlist {
            match get_video_title(url).await {
                Ok(video_title) => {
                    // Check if video already exists
                    if let Some(existing_file) = check_if_video_exists(&download_dir, format, &video_title) {
                        println!("{}: {:?}", "Found existing download".yellow(), existing_file);
                        
                        // Prompt user if they want to download again
                        if !prompt_for_redownload()? {
                            println!("{}", "Download cancelled.".green());
                            return Ok(());
                        }
                        
                        // If yes, set flag to use unique filename
                        should_use_unique_filename = true;
                        println!("{}: Will append timestamp to filename", "Duplicate download".blue());
                    }
                },
                Err(e) => {
                    println!("{}: {}", "Warning: Could not get video title".yellow(), e);
                    println!("{}", "Proceeding with download without duplicate check...".yellow());
                }
            }
        }
    }

    // If force_download is enabled, clear any partial downloads
    if force_download {
        println!("{}", "Force download mode enabled - clearing partial downloads".blue());
        if let Err(e) = clear_partial_downloads(url) {
            println!("{}", format!("Warning: Could not clear partial downloads: {}. Continuing anyway.", e).yellow());
        }
    }

    // Validate time formats if provided
    if let Some(start) = start_time {
        validate_time_format(start)?;
    }

    if let Some(end) = end_time {
        validate_time_format(end)?;
    }

    // Validate bitrate if provided
    if let Some(rate) = bitrate {
        validate_bitrate(rate)?;

        // For video, we can respect the bitrate in free version
        // For audio, we enforce the free version limitation
        if format != "mp3" {
            println!("{}: {}", "Video bitrate".blue(), rate);
        }
    }

    // Apply quality selection - no limitations in free version now
    let _limited_quality = quality.map(limit_video_quality);

    // Create the output path format with validation
    let output_path = if should_use_unique_filename {
        format_output_path_with_timestamp(&download_dir, format, &timestamp)?
    } else {
        format_output_path(&download_dir, format)?
    };

    // Create enhanced progress tracking
    let progress = Arc::new(DownloadProgress::new());

    // Create progress bar with more detailed template
    let pb = Arc::new(ProgressBar::new(100));
    // Instead of using set_suffix, modify the ProgressBar template to include sections for these values
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    // Then update the message with all the information
    pb.set_message(format!("Size: {} | Speed: {} | ETA: {}", 
        "Calculating...", "Connecting...", "Calculating..."));

    // Show a promo message during download preparation
    display_download_promo();

    // Build yt-dlp command securely
    let mut command = AsyncCommand::new("yt-dlp");

    // Enable parallel downloads (replacing the single-threaded restriction)
    enable_parallel_download(&mut command);

    // If force download is enabled, don't try to resume partial downloads
    if force_download {
        command.arg("--no-continue");  // Don't try to resume partial downloads
        command.arg("--no-part-file"); // Don't create .part files
    }

    // Add format selection based on requested format and quality
    if format == "mp3" && !is_ffmpeg_available() {
        let ffmpeg_available = match std::process::Command::new("ffmpeg").arg("-version").output() {
            Ok(_) => true,
            Err(_) => {
                println!("{}", "⚠️ Warning: FFmpeg not found but required for audio conversion. ⚠️".bright_red());
                println!("{}", "Will attempt to continue, but audio extraction may fail.".bright_red());
                false
            },
        };
        
        // Add format selection and ffmpeg args
        command.arg("-f")
            .arg("bestaudio[ext=m4a]")
            .arg("--extract-audio")
            .arg("--audio-format")
            .arg("mp3");
            
        // Apply audio quality limitation for free version
        modify_audio_command(&mut command, bitrate);
        
        // If ffmpeg is not available, warn user again
        if !ffmpeg_available {
            println!("{}", "🔴 Attempting audio extraction without verified FFmpeg - this may fail".bright_red());
        }
    }

    // Escape the output path properly
    command.arg("-o").arg(&output_path);

    // Handle playlist options
    if use_playlist {
        command.arg("--yes-playlist");
        println!("{}", "Playlist mode enabled - will download all videos in playlist".yellow());
    } else {
        command.arg("--no-playlist");
    }

    // Add subtitles if requested
    if download_subtitles {
        command.arg("--write-subs").arg("--sub-langs").arg("all");
        println!("{}", "Subtitles will be downloaded if available".blue());
    }

    // Process start and end times with enhanced security
    if start_time.is_some() || end_time.is_some() {
        // Check if ffmpeg is available
        let ffmpeg_available = match std::process::Command::new("ffmpeg").arg("-version").output() {
            Ok(_) => true,
            Err(_) => false,
        };
        
        if !ffmpeg_available {
            println!("{}", "⚠️ Warning: FFmpeg not found. Time-based extraction might not work. ⚠️".yellow());
            println!("{}", "Will attempt to continue, but the operation may fail.".yellow());
        }
        
        let mut time_args = String::new();

        if let Some(start) = start_time {
            // Validate again right before using
            validate_time_format(start)?;
            time_args.push_str(&format!("-ss {} ", start));
        }

        if let Some(end) = end_time {
            // Validate again right before using
            validate_time_format(end)?;
            time_args.push_str(&format!("-to {} ", end));
        }

        if !time_args.is_empty() {
            command.arg("--postprocessor-args").arg(format!("ffmpeg:{}", time_args.trim()));
        }
    }

    // Add throttling and retry options to avoid detection
    command.arg("--socket-timeout").arg("30");
    command.arg("--retries").arg("10");
    command.arg("--fragment-retries").arg("10");
    command.arg("--throttled-rate").arg("100K");

    // Add progress output format for parsing
    command.arg("--newline");
    command.arg("--progress-template").arg("download:%(progress.downloaded_bytes)s/%(progress.total_bytes)s");

    // Add user agent to avoid detection
    command.arg("--user-agent")
        .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

    // Add the URL last
    command.arg(url);

    // Execute the command
    println!("{}", "Starting download...".green());

    // Set up pipes for stdout and stderr
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    // Spawn the command with improved error handling
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            match e.kind() {
                io::ErrorKind::NotFound => {
                    eprintln!("{}", "Error: yt-dlp executable not found. Please ensure it's installed and in your PATH.".red());
                    return Err(AppError::MissingDependency("yt-dlp".to_string()));
                },
                io::ErrorKind::PermissionDenied => {
                    eprintln!("{}", "Error: Permission denied when running yt-dlp. Check your file permissions.".red());
                    return Err(AppError::IoError(e));
                },
                _ => {
                    eprintln!("{}", format!("Failed to execute yt-dlp command: {}. Check your network connection.", e).red());
                    return Err(AppError::IoError(e));
                }
            }
        }
    };

    // Process stdout to update progress bar with enhanced information
    if let Some(stdout) = child.stdout.take() {
        let stdout_reader = BufReader::new(stdout);
        let mut lines = stdout_reader.lines();
        let pb_clone = Arc::clone(&pb);
        let progress_clone = Arc::clone(&progress);

        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if line.starts_with("download:") {
                    if let Some(progress_str) = line.strip_prefix("download:") {
                        let parts: Vec<&str> = progress_str.split('/').collect();
                        if parts.len() == 2 {
                            // Try to parse downloaded and total bytes
                            if let (Ok(downloaded), Ok(total)) = (
                                parts[0].trim().parse::<u64>(),
                                parts[1].trim().parse::<u64>(),
                            ) {
                                if total > 0 {
                                    // Update progress tracking
                                    progress_clone.update(downloaded, total);
                                    
                                    // Update progress bar
                                    let percentage = progress_clone.get_percentage();
                                    pb_clone.set_position(percentage);
                                    
                                    // Update size, speed and ETA information
                                    pb_clone.set_message(format!("Size: {} | Speed: {} | ETA: {}",
                                        progress_clone.format_file_size(),
                                        progress_clone.format_speed(),
                                        progress_clone.format_eta()));
                                }
                            }
                        }
                    }
                } else {
                    // Print other output from yt-dlp
                    println!("{}", line);
                }
            }
        });
    }

    // Process stderr to show errors
    if let Some(stderr) = child.stderr.take() {
        let stderr_reader = BufReader::new(stderr);
        let mut lines = stderr_reader.lines();

        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                // Check for HTTP 416 error specifically
                if line.contains("HTTP Error 416") || line.contains("Requested Range Not Satisfiable") {
                    eprintln!("{}", "Error: File already exists or download was previously completed.".red());
                } else {
                    eprintln!("{}", line.red());
                }
            }
        });
    }

    // Wait for the command to finish with enhanced error handling
    let status = match child.wait().await {
        Ok(status) => status,
        Err(e) => {
            match e.kind() {
                io::ErrorKind::BrokenPipe => {
                    eprintln!("{}", "Error: Connection interrupted. Check your network connection and try again.".red());
                    return Err(AppError::IoError(e));
                },
                io::ErrorKind::TimedOut => {
                    eprintln!("{}", "Error: Connection timed out. The server might be busy or your connection is slow.".red());
                    return Err(AppError::IoError(e));
                },
                _ => {
                    eprintln!("{}", format!("Failed to complete download: {}. Check your network connection.", e).red());
                    return Err(AppError::IoError(e));
                }
            }
        }
    };

    // Finish the progress bar
    pb.finish_with_message("Download completed");

    // Check if command succeeded
    if !status.success() {
        // We can't check stderr_output as it's processed asynchronously
        // Instead, use a more generic error message with hints
        let exit_code = status.code().unwrap_or(0);
        
        if exit_code == 1 && format == "mp3" && !is_ffmpeg_available() {
            // If we're doing audio extraction, ffmpeg isn't available, and we got exit code 1
            return Err(AppError::DownloadError(
                "Download failed, likely due to missing or incompatible ffmpeg. Please install ffmpeg and try again.".to_string(),
            ));
        } else if start_time.is_some() || end_time.is_some() {
            // If we're doing time extraction and the command failed
            return Err(AppError::DownloadError(
                "Download with time selection failed. This feature requires a working ffmpeg installation.".to_string(),
            ));
        } else {
            // General failure
            return Err(AppError::DownloadError(
                format!("yt-dlp command failed with exit code {}. Please verify the URL and options provided.", exit_code)
            ));
        }
    }

    // Increment download counter if not using force_download
    if !force_download {
        counter.increment()?;
    }

    // Send desktop notification
    let notification_result = Notification::new()
        .summary("Download Complete")
        .body(&format!("{} file downloaded successfully.", format.to_uppercase()))
        .show();

    // Handle notification errors separately so they don't prevent download completion
    if let Err(e) = notification_result {
        println!("{}: {}", "Failed to show notification".yellow(), e);
    }

    println!(
        "{} {} {}",
        "Download completed successfully.".green(),
        format.to_uppercase(),
        "file saved.".green()
    );

    // Show completion promo
    display_completion_promo();

    Ok(())
}