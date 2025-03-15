use crate::error::AppError;
use crate::utils::{format_output_path, initialize_download_dir, validate_bitrate, validate_path_safety, validate_time_format, validate_url};
use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use colored::*;
use dirs_next as dirs;
use hostname;
use humansize::{format_size, BINARY};
use indicatif::{ProgressBar, ProgressStyle};
use notify_rust::Notification;
use once_cell::sync::Lazy;
use rand::Rng;
use regex::Regex;
use ring::{digest, hmac};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as AsyncCommand;

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

struct DownloadProgress {
    start_time: Instant,
    last_update: Mutex<Instant>,
    downloaded_bytes: AtomicU64,
    total_bytes: AtomicU64,
    download_speed: Mutex<f64>,
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
        let current_downloaded = self.downloaded_bytes.load(Ordering::SeqCst);
        
        let bytes_diff = if downloaded > current_downloaded {
            downloaded - current_downloaded
        } else {
            0
        };
        
        self.downloaded_bytes.store(downloaded, Ordering::SeqCst);
        self.total_bytes.store(total, Ordering::SeqCst);

        let now = Instant::now();
        let mut last_update = self.last_update.lock().unwrap();
        let time_diff = now.duration_since(*last_update).as_millis();

        if time_diff >= 100 && bytes_diff > 0 {
            let mut last_speed_samples = self.last_speed_samples.lock().unwrap();
            let mut speed = self.download_speed.lock().unwrap();

            let current_speed = bytes_diff as f64 / (time_diff as f64 / 1000.0);
            
            last_speed_samples.push(current_speed);
            if last_speed_samples.len() > 10 {
                last_speed_samples.remove(0);
            }
            
            if !last_speed_samples.is_empty() {
                let sum: f64 = last_speed_samples.iter().sum();
                *speed = sum / last_speed_samples.len() as f64;
            }

            *last_update = now;
        }
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
    
        command.arg("--concurrent-fragments").arg("4");
        
        let aria2c_available = std::process::Command::new("aria2c")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        if aria2c_available {
            command.arg("--downloader").arg("aria2c");
            command.arg("--downloader-args").arg("aria2c:-x4");
            command.arg("--downloader-args").arg("aria2c:-k1M");
        } else {
            command.arg("--downloader").arg("yt-dlp");
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
        for entry_result in entries {
            if let Ok(entry) = entry_result {
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

    let output = command.output().await.map_err(|e| AppError::IoError(e))?;

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
        },
        Err(_) => None,
    }
}

fn prompt_for_redownload() -> Result<bool, AppError> {
    print!("This video has already been downloaded. Do you want to download it again? (y/n): ");
    io::stdout().flush().map_err(|e| AppError::IoError(e))?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| AppError::IoError(e))?;

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
    
    let command = YtdlpCommandBuilder::new(url, &output_path)
        .with_format(format)
        .with_quality(quality)
        .with_time_range(start_time, end_time)
        .with_playlist(use_playlist)
        .with_subtitles(download_subtitles)
        .with_force_download(force_download)
        .with_bitrate(bitrate)
        .build();

    println!("{}", "Starting download...".green());

    let mut child = match command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(e) => {
            match e.kind() {
                io::ErrorKind::NotFound => {
                    eprintln!("{}", "Error: yt-dlp executable not found. Please ensure it's installed and in your PATH.".red());
                    return Err(AppError::MissingDependency("yt-dlp".to_string()));
                }
                io::ErrorKind::PermissionDenied => {
                    eprintln!("{}", "Error: Permission denied when running yt-dlp. Check your file permissions.".red());
                    return Err(AppError::IoError(e));
                }
                _ => {
                    eprintln!("{}", format!("Failed to execute yt-dlp command: {}. Check your network connection.", e).red());
                    return Err(AppError::IoError(e));
                }
            }
        }
    };

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
                            if let (Ok(downloaded), Ok(total)) = (
                                parts[0].trim().parse::<u64>(),
                                parts[1].trim().parse::<u64>(),
                            ) {
                                if total > 0 {
                                    progress_clone.update(downloaded, total);
                                    let percentage = progress_clone.get_percentage();
                                    pb_clone.set_position(percentage);
                                    pb_clone.set_message(format!(
                                        "Size: {} | Speed: {} | ETA: {}",
                                        progress_clone.format_file_size(),
                                        progress_clone.format_speed(),
                                        progress_clone.format_eta()
                                    ));
                                }
                            }
                        }
                    }
                } else {
                    println!("{}", line);
                }
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let stderr_reader = BufReader::new(stderr);
        let mut lines = stderr_reader.lines();

        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if line.contains("HTTP Error 416") || line.contains("Requested Range Not Satisfiable") {
                    eprintln!("{}", "Error: File already exists or download was previously completed.".red());
                } else {
                    eprintln!("{}", line.red());
                }
            }
        });
    }

    let status = match child.wait().await {
        Ok(status) => status,
        Err(e) => {
            match e.kind() {
                io::ErrorKind::BrokenPipe => {
                    eprintln!("{}", "Error: Connection interrupted. Check your network connection and try again.".red());
                    return Err(AppError::IoError(e));
                }
                io::ErrorKind::TimedOut => {
                    eprintln!("{}", "Error: Connection timed out. The server might be busy or your connection is slow.".red());
                    return Err(AppError::IoError(e));
                }
                _ => {
                    eprintln!("{}", format!("Failed to complete download: {}. Check your network connection.", e).red());
                    return Err(AppError::IoError(e));
                }
            }
        }
    };

    pb.finish_with_message("Download completed");

    if !status.success() {
        let exit_code = status.code().unwrap_or(0);

        if exit_code == 1 && format == "mp3" && !*FFMPEG_AVAILABLE {
            return Err(AppError::DownloadError(
                "Download failed, likely due to missing or incompatible ffmpeg. Please install ffmpeg and try again.".to_string(),
            ));
        } else if start_time.is_some() || end_time.is_some() {
            return Err(AppError::DownloadError(
                "Download with time selection failed. This feature requires a working ffmpeg installation.".to_string(),
            ));
        } else {
            return Err(AppError::DownloadError(
                format!("yt-dlp command failed with exit code {}. Please verify the URL and options provided.", exit_code)
            ));
        }
    }

    if !force_download {
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