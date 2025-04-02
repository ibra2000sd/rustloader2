// src/utils.rs

use crate::error::AppError;
use base64::{engine::general_purpose, Engine as _};
use colored::*;
use home::home_dir;
use regex::Regex;
use reqwest::Client;
use ring::signature;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ShellCommand, Stdio};

/// Validate path to prevent path traversal attacks
pub fn validate_path_safety(path: &Path) -> Result<(), AppError> {
    crate::security::validate_path_safety(path)
}

/// Check path components for relative traversal attempts
#[allow(dead_code)]
fn check_path_components(path: &Path) -> Result<(), AppError> {
    let path_str = path.to_string_lossy();
    if path_str.contains("../")
        || path_str.contains("..\\")
        || path_str.contains("/..")
        || path_str.contains("\\..")
        || path_str.contains("~")
    {
        return Err(AppError::SecurityViolation);
    }
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(AppError::SecurityViolation);
            }
            _ => continue,
        }
    }
    Ok(())
}

/// Check if a dependency is installed by searching for it in PATH
#[allow(dead_code)]
pub fn is_dependency_installed(name: &str) -> Result<bool, AppError> {
    let command = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };

    let output = ShellCommand::new(command)
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(AppError::IoError)?;

    Ok(output.success())
}

/// Get the version of a dependency
#[allow(dead_code)]
pub fn get_dependency_version(name: &str) -> Result<String, AppError> {
    let output = ShellCommand::new(name)
        .arg("--version")
        .output()
        .map_err(AppError::IoError)?;

    if !output.status.success() {
        return Err(AppError::General(format!("Failed to get {} version", name)));
    }

    let version_output = String::from_utf8_lossy(&output.stdout).to_string();
    let version = version_output
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(version)
}

/// Check if yt-dlp is up to date
#[allow(dead_code)]
pub fn is_ytdlp_updated() -> Result<bool, AppError> {
    let output = ShellCommand::new("yt-dlp")
        .arg("--update")
        .output()
        .map_err(AppError::IoError)?;

    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(output_str.contains("is up to date") || output_str.contains("Updated"))
}

/// Update yt-dlp to latest version
#[allow(dead_code)]
pub fn update_ytdlp() -> Result<(), AppError> {
    println!("{}", "Updating yt-dlp...".blue());
    let output = ShellCommand::new("yt-dlp")
        .arg("--update")
        .status()
        .map_err(AppError::IoError)?;

    if output.success() {
        println!("{}", "yt-dlp updated successfully.".green());
        Ok(())
    } else {
        eprintln!("{}", "Failed to update yt-dlp.".red());
        Err(AppError::General("yt-dlp update failed".to_string()))
    }
}

/// Check if all required dependencies are installed and up to date
#[allow(dead_code)]
pub fn check_dependencies() -> Result<(), AppError> {
    if !is_dependency_installed("yt-dlp")? {
        eprintln!(
            "{}",
            "yt-dlp is not installed. Please install it and try again.".red()
        );
        return Err(AppError::MissingDependency("yt-dlp".to_string()));
    }

    println!("{}", "Checking if yt-dlp is up to date...".blue());
    match is_ytdlp_updated() {
        Ok(true) => println!("{}", "yt-dlp is up to date.".green()),
        Ok(false) => {
            println!("{}", "yt-dlp needs to be updated.".yellow());
            update_ytdlp()?;
        }
        Err(e) => {
            println!(
                "{}",
                format!("Could not check yt-dlp version: {}. Continuing anyway.", e).yellow()
            );
        }
    }

    if !is_dependency_installed("ffmpeg")? {
        eprintln!("{}", "ffmpeg is not installed.".yellow());
        return Err(AppError::MissingDependency("ffmpeg".to_string()));
    }

    match get_dependency_version("ffmpeg") {
        Ok(version) => println!("{} {}", "ffmpeg version:".blue(), version),
        Err(_) => println!(
            "{}",
            "Could not determine ffmpeg version. Continuing anyway.".yellow()
        ),
    }

    Ok(())
}

/// Install ffmpeg based on the current operating system
#[allow(dead_code)]
pub fn install_ffmpeg() -> Result<(), AppError> {
    println!("{}", "Installing ffmpeg...".blue());

    #[cfg(target_os = "macos")]
    {
        let status = ShellCommand::new("brew")
            .arg("install")
            .arg("ffmpeg")
            .status()
            .map_err(AppError::IoError)?;

        if status.success() {
            println!("{}", "ffmpeg installed successfully.".green());
        } else {
            eprintln!(
                "{}",
                "Failed to install ffmpeg. Please install it manually.".red()
            );
            return Err(AppError::General("ffmpeg installation failed.".to_string()));
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = ShellCommand::new("sudo")
            .arg("apt")
            .arg("install")
            .arg("-y")
            .arg("ffmpeg")
            .status()
            .map_err(AppError::IoError)?;

        if status.success() {
            println!("{}", "ffmpeg installed successfully.".green());
        } else {
            eprintln!(
                "{}",
                "Failed to install ffmpeg. Please install it manually.".red()
            );
            return Err(AppError::General("ffmpeg installation failed.".to_string()));
        }
    }

    #[cfg(target_os = "windows")]
    {
        println!(
            "{}",
            "Automatic installation of ffmpeg is not supported on Windows.".yellow()
        );
        println!(
            "{}",
            "Please download and install ffmpeg manually from: https://ffmpeg.org/download.html"
                .yellow()
        );
        return Err(AppError::General(
            "Automatic ffmpeg installation not supported on Windows.".to_string(),
        ));
    }

    Ok(())
}

/// Modified validate_url function with adjusted checks to allow encoded URLs
pub fn validate_url(url: &str) -> Result<(), AppError> {
    // Apply rate limiting to URL validation to prevent DoS
    if !crate::security::apply_rate_limit("url_validation", 20, std::time::Duration::from_secs(60))
    {
        return Err(AppError::ValidationError(
            "Too many validation attempts. Please try again later.".to_string(),
        ));
    }

    // Check for common URLs we want to support first
    let youtube_regex = Regex::new(r"^https?://(?:www\.)?(?:youtube\.com|youtu\.be)/").unwrap();
    let vimeo_regex = Regex::new(r"^https?://(?:www\.)?vimeo\.com/").unwrap();
    let dailymotion_regex = Regex::new(r"^https?://(?:www\.)?dailymotion\.com/").unwrap();

    if youtube_regex.is_match(url) || vimeo_regex.is_match(url) || dailymotion_regex.is_match(url) {
        println!("{}", "URL validated as known video platform".green());
        return Ok(());
    }

    // More generic URL validation for other sites
    let url_regex = Regex::new(
        r"^https?://(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\+.~#?&//=]*)$"
    ).unwrap();
    if !url_regex.is_match(url) {
        return Err(AppError::ValidationError(format!(
            "Invalid URL format: {}",
            url
        )));
    }

    // Enhanced security check: detect command injection
    if crate::security::detect_command_injection(url) {
        println!(
            "{}",
            "⚠️ Security violation detected in URL ⚠️".bright_red()
        );
        return Err(AppError::SecurityViolation);
    }

    // Check URL length to prevent DoS attacks
    if url.len() > 4096 {
        return Err(AppError::ValidationError(
            "URL exceeds maximum allowed length".to_string(),
        ));
    }

    // Check only for truly problematic characters
    let unusual_chars = ['<', '>', '\\', '{', '}', '^'];
    let has_unusual_chars = url.chars().any(|c| unusual_chars.contains(&c));
    if has_unusual_chars {
        return Err(AppError::ValidationError(
            "URL contains unusual characters".to_string(),
        ));
    }

    // Validate URL does not target internal network
    let localhost_patterns = [
        "localhost",
        "127.0.0.1",
        "::1",
        "0.0.0.0",
        "10.",
        "192.168.",
        "172.16.",
    ];
    if localhost_patterns
        .iter()
        .any(|&pattern| url.contains(pattern))
    {
        return Err(AppError::ValidationError(
            "URLs targeting internal networks are not allowed".to_string(),
        ));
    }

    Ok(())
}

/// Validate time format (HH:MM:SS)
pub fn validate_time_format(time: &str) -> Result<(), AppError> {
    let re = Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap();
    if !re.is_match(time) {
        return Err(AppError::TimeFormatError(
            "Time must be in the format HH:MM:SS".to_string(),
        ));
    }

    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 3 {
        return Err(AppError::TimeFormatError(
            "Time must have hours, minutes, and seconds components".to_string(),
        ));
    }

    let hours: u32 = parts[0]
        .parse()
        .map_err(|_| AppError::TimeFormatError("Hours must be a valid number".to_string()))?;
    let minutes: u32 = parts[1]
        .parse()
        .map_err(|_| AppError::TimeFormatError("Minutes must be a valid number".to_string()))?;
    let seconds: u32 = parts[2]
        .parse()
        .map_err(|_| AppError::TimeFormatError("Seconds must be a valid number".to_string()))?;

    if hours >= 24 {
        return Err(AppError::TimeFormatError(
            "Hours must be between 00-23".to_string(),
        ));
    }
    if minutes >= 60 {
        return Err(AppError::TimeFormatError(
            "Minutes must be between 00-59".to_string(),
        ));
    }
    if seconds >= 60 {
        return Err(AppError::TimeFormatError(
            "Seconds must be between 00-59".to_string(),
        ));
    }

    Ok(())
}

/// Validate the provided bitrate format (e.g., 1000K)
pub fn validate_bitrate(bitrate: &str) -> Result<(), AppError> {
    let re = Regex::new(r"^(\d+)(K|M)$").unwrap();
    if !re.is_match(bitrate) {
        return Err(AppError::ValidationError(format!(
            "Invalid bitrate format: {}. Use format like '1000K' or '5M'",
            bitrate
        )));
    }

    if let Some(captures) = re.captures(bitrate) {
        let value = captures.get(1).unwrap().as_str();
        let value_num: u32 = match value.parse() {
            Ok(num) => num,
            Err(_) => {
                return Err(AppError::ValidationError(format!(
                    "Invalid bitrate value: {}. Must be a valid number.",
                    value
                )));
            }
        };
        if value_num == 0 {
            return Err(AppError::ValidationError(
                "Bitrate cannot be zero.".to_string(),
            ));
        }

        let unit = captures.get(2).unwrap().as_str();
        if unit == "K" && value_num > 10000 {
            return Err(AppError::ValidationError(
                "Bitrate too high (max 10000K)".to_string(),
            ));
        } else if unit == "M" && value_num > 100 {
            return Err(AppError::ValidationError(
                "Bitrate too high (max 100M)".to_string(),
            ));
        }
    }

    Ok(())
}

/// Enhanced initialize_download_dir with security checks
pub fn initialize_download_dir(
    custom_dir: Option<&str>,
    program_name: &str,
    file_type: &str,
) -> Result<PathBuf, AppError> {
    let download_dir = if let Some(dir) = custom_dir {
        let path = PathBuf::from(dir);
        validate_path_safety(&path)?;
        path
    } else {
        match home_dir() {
            Some(mut path) => {
                path.push("Downloads");
                path.push(program_name);
                path.push(file_type);
                validate_path_safety(&path)?;
                path
            }
            None => {
                return Err(AppError::PathError(
                    "Failed to find the home directory.".to_string(),
                ));
            }
        }
    };

    if !download_dir.exists() {
        fs::create_dir_all(&download_dir).map_err(|e| {
            eprintln!("{}: {:?}", "Failed to create download directory".red(), e);
            AppError::IoError(e)
        })?;
        println!("{} {:?}", "Created directory:".green(), download_dir);
    }

    Ok(download_dir)
}

/// Sanitize a path string using a strict whitelist approach
fn sanitize_path(path: &str) -> Result<String, AppError> {
    let path_obj = std::path::Path::new(path);
    let dir_part = if let Some(parent) = path_obj.parent() {
        let dir_str = parent.to_string_lossy();
        if dir_str.contains("..")
            || dir_str.contains('~')
            || dir_str.contains('*')
            || dir_str.contains('?')
            || dir_str.contains('|')
            || dir_str.contains(';')
            || dir_str.contains('&')
            || dir_str.contains('<')
            || dir_str.contains('>')
        {
            return Err(AppError::ValidationError(
                "Directory path contains invalid characters".to_string(),
            ));
        }
        dir_str.to_string()
    } else {
        String::new()
    };

    let file_part = if let Some(file_name) = path_obj.file_name() {
        let file_str = file_name.to_string_lossy();
        let sanitized_file: String = file_str
            .chars()
            .filter(|c| {
                c.is_ascii_alphanumeric()
                    || *c == '.'
                    || *c == '-'
                    || *c == '_'
                    || *c == ' '
                    || *c == '('
                    || *c == ')'
                    || *c == '%'
            })
            .collect();

        if sanitized_file.len() < file_str.len() * 3 / 4 {
            return Err(AppError::ValidationError(
                "Filename contains too many invalid characters".to_string(),
            ));
        }
        sanitized_file
    } else {
        return Err(AppError::ValidationError("No filename in path".to_string()));
    };

    if dir_part.is_empty() {
        Ok(file_part)
    } else {
        let separator = if cfg!(windows) { "\\" } else { "/" };
        Ok(format!("{}{}{}", dir_part, separator, file_part))
    }
}

/// Format a safe path for use with yt-dlp
pub fn format_output_path<P: AsRef<Path>>(
    download_dir: P,
    format: &str,
) -> Result<String, AppError> {
    validate_path_safety(download_dir.as_ref())?;
    match format {
        "mp3" | "mp4" | "webm" | "m4a" | "flac" | "wav" | "ogg" => {}
        _ => {
            return Err(AppError::ValidationError(format!(
                "Invalid output format: {}",
                format
            )))
        }
    }

    let path_buf = download_dir.as_ref().join(format!("%(title)s.{}", format));
    let path_str = path_buf
        .to_str()
        .ok_or_else(|| AppError::PathError("Invalid path encoding".to_string()))?
        .to_string();

    let sanitized_path = sanitize_path(&path_str)?;
    Ok(sanitized_path)
}

#[derive(Deserialize, Debug)]
struct SignedReleaseInfo {
    release: ReleaseInfo,
    signature: String,
    pub_key_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReleaseInfo {
    tag_name: String,
    html_url: String,
    prerelease: bool,
    release_notes: String,
    release_date: String,
    checksum: String,
}

struct TrustedKeys {
    keys: Vec<(String, Vec<u8>)>,
    key_expiry: std::collections::HashMap<String, i64>,
    last_refresh: std::time::Instant,
}

impl TrustedKeys {
    fn new() -> Self {
        let mut key_expiry = std::collections::HashMap::new();
        let expiry_time = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::days(180))
            .unwrap_or_else(chrono::Utc::now)
            .timestamp();
        key_expiry.insert("rustloader-release-key-1".to_string(), expiry_time);

        Self {
            keys: vec![
                (
                    "rustloader-release-key-1".to_string(), 
                    general_purpose::STANDARD.decode(
                        "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAzm8X3PIzQAHU0QN9JV9TOT+1F5iHnJXUm"
                    ).unwrap_or_default()
                ),
                (
                    "rustloader-backup-key-1".to_string(), 
                    general_purpose::STANDARD.decode(
                        "MIIBCgKCAQEAybvA4wNZm3VRpjBMIpxmRvwP9H4mj5YwbkrDraIiu95BU3yU+"
                    ).unwrap_or_default()
                ),
            ],
            key_expiry,
            last_refresh: std::time::Instant::now(),
        }
    }

    fn get_key_by_id(&self, key_id: &str) -> Option<&Vec<u8>> {
        if let Some(expiry) = self.key_expiry.get(key_id) {
            let current_time = chrono::Utc::now().timestamp();
            if current_time > *expiry {
                println!(
                    "{}",
                    "Warning: Update signature key has expired. Please update Rustloader.".red()
                );
                return None;
            }
        }
        self.keys
            .iter()
            .find(|(id, _)| id == key_id)
            .map(|(_, key)| key)
    }

    #[allow(dead_code)]
    fn refresh_keys(&mut self) -> Result<(), AppError> {
        if self.last_refresh.elapsed() < std::time::Duration::from_secs(86400) {
            return Ok(());
        }
        self.last_refresh = std::time::Instant::now();
        Ok(())
    }
}

fn verify_release_signature(
    data: &ReleaseInfo,
    signature: &str,
    public_key: &[u8],
) -> Result<bool, AppError> {
    let data_json = serde_json::to_string(data)?;
    let signature_bytes = match general_purpose::STANDARD.decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false),
    };
    match verify_signature(data_json.as_bytes(), &signature_bytes, public_key) {
        Ok(valid) => Ok(valid),
        Err(_) => Ok(false),
    }
}

fn verify_signature(data: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, AppError> {
    let public_key =
        signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, public_key);
    match public_key.verify(data, signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

pub async fn check_for_updates() -> Result<bool, AppError> {
    let current_version = match Version::parse(crate::version::VERSION) {
        Ok(v) => v,
        Err(_) => {
            return Err(AppError::General(
                "Invalid current version format".to_string(),
            ))
        }
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .https_only(true)
        .build()?;

    let url = "https://api.rustloader.com/releases/latest";
    let response = match client
        .get(url)
        .header("User-Agent", format!("rustloader/{}", crate::version::VERSION))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            println!("{} {}", "Could not check for updates:".yellow(), e);
            return Ok(false);
        }
    };

    if response.status().is_success() {
        match response.json::<SignedReleaseInfo>().await {
            Ok(signed_release) => {
                if signed_release.release.prerelease {
                    return Ok(false);
                }
                let trusted_keys = TrustedKeys::new();
                if let Some(public_key) = trusted_keys.get_key_by_id(&signed_release.pub_key_id) {
                    let signature_valid = verify_release_signature(
                        &signed_release.release,
                        &signed_release.signature,
                        public_key,
                    )?;
                    if !signature_valid {
                        println!("{}", "Update signature verification failed!".red());
                        return Ok(false);
                    }
                } else {
                    println!("{}", "Update signed with untrusted key!".red());
                    return Ok(false);
                }
                let version_str = signed_release.release.tag_name.trim_start_matches('v');
                match Version::parse(version_str) {
                    Ok(latest_version) => {
                        if latest_version > current_version {
                            println!(
                                "{} {} -> {}",
                                "New version available:".bright_yellow(),
                                current_version,
                                latest_version
                            );
                            println!(
                                "{} {}",
                                "Download at:".bright_yellow(),
                                signed_release.release.html_url
                            );
                            println!(
                                "{} {}",
                                "Release notes:".bright_cyan(),
                                signed_release.release.release_notes
                            );
                            println!(
                                "{} {}",
                                "SHA-256 checksum:".bright_cyan(),
                                signed_release.release.checksum
                            );
                            return Ok(true);
                        }
                    }
                    Err(_) => {
                        return Ok(false);
                    }
                }
            }
            Err(_) => {
                return Ok(false);
            }
        }
    }
    Ok(false)
}
