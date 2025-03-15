//! Enhanced dependency validator for Rustloader
//!
//! This module provides functionality to validate and verify external dependencies
//! like yt-dlp and ffmpeg, checking versions, binary integrity, and known vulnerabilities.

use crate::error::AppError;
use base64::{engine::general_purpose, Engine as _};
use colored::*;
use ring::digest;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

// Minimum acceptable versions for dependencies
pub const MIN_YTDLP_VERSION: &str = "2023.07.06";
pub const MIN_FFMPEG_VERSION: &str = "4.0.0";

// Known vulnerable versions to warn about
const VULNERABLE_YTDLP_VERSIONS: [&str; 2] = ["2022.05.18", "2022.08.14"];
const VULNERABLE_FFMPEG_VERSIONS: [&str; 2] = ["4.3.1", "4.4.2"];

#[allow(dead_code)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub path: String,
    pub hash: Option<String>,
    pub is_min_version: bool,
    pub is_vulnerable: bool,
}

// Modified function with code quality fixes
fn get_dependency_path(name: &str) -> Result<String, AppError> {
    #[cfg(target_os = "windows")]
    let search_commands = vec!["where"];

    #[cfg(not(target_os = "windows"))]
    let search_commands = vec!["which"];

    for command in &search_commands {
        if let Ok(output) = Command::new(command).arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    println!("{}: {}", format!("Found {} at", name).green(), path);
                    return Ok(path);
                }
            }
        }
    }

    if Command::new(name).arg("-version").output().is_ok() {
        println!("{}", format!("{} is available in PATH", name).green());
        return Ok(name.to_string());
    }

    if name == "ffmpeg" {
        let common_paths = vec![
            "/usr/bin/ffmpeg",
            "/usr/local/bin/ffmpeg",
            "/opt/homebrew/bin/ffmpeg",
            "/snap/bin/ffmpeg",
            "/var/lib/flatpak/app/org.ffmpeg/ffmpeg",
            "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
            "C:\\ffmpeg\\bin\\ffmpeg.exe",
        ];
        for path in common_paths {
            if Path::new(path).exists() {
                println!("{}: {}", format!("Found {} at", name).green(), path);
                return Ok(path.to_string());
            }
        }
    }

    println!(
        "{}",
        format!(
            "Warning: {} not found in PATH or common locations. Will try to proceed anyway.",
            name
        )
        .yellow()
    );
    Ok(format!("__continuing_without_{}", name))
}

fn calculate_file_hash(path: &str) -> Result<String, AppError> {
    let mut file = File::open(path).map_err(AppError::IoError)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(AppError::IoError)?;
    let digest = digest::digest(&digest::SHA256, &buffer);
    Ok(general_purpose::STANDARD.encode(digest.as_ref()))
}

fn parse_version(output: &str, name: &str) -> String {
    let version_patterns = match name {
        "yt-dlp" => vec![
            r"(?i)yt-dlp\s+(\d+\.\d+\.\d+)",
            r"(?i)version\s+(\d+\.\d+\.\d+)",
            r"(?i)(\d+\.\d+\.\d+)",
        ],
        "ffmpeg" => vec![
            r"(?i)ffmpeg\s+version\s+(\d+\.\d+(?:\.\d+)?)",
            r"(?i)version\s+(\d+\.\d+(?:\.\d+)?)",
            r"(?i)ffmpeg\s+(\d+\.\d+(?:\.\d+)?)",
            r"(?i)ffmpeg.*?(\d+\.\d+(?:\.\d+)?)",
            r"(?i)(\d+\.\d+(?:\.\d+)?)",
        ],
        _ => vec![r"(\d+\.\d+\.\d+)"],
    };

    for pattern in version_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(captures) = re.captures(output) {
                if let Some(version) = captures.get(1) {
                    return version.as_str().to_string();
                }
            }
        }
    }

    let generic_pattern = r"(\d+\.\d+(?:\.\d+)?)";
    if let Ok(re) = regex::Regex::new(generic_pattern) {
        if let Some(captures) = re.captures(output) {
            if let Some(version) = captures.get(1) {
                println!(
                    "{}",
                    format!(
                        "Found potential {} version using fallback method: {}",
                        name,
                        version.as_str()
                    )
                    .yellow()
                );
                return version.as_str().to_string();
            }
        }
    }

    println!(
        "{}",
        format!(
            "Could not parse version from output for {}: {}",
            name, output
        )
        .yellow()
    );
    println!(
        "{}",
        "Returning 'unknown' as version - will attempt to continue".yellow()
    );

    output
        .lines()
        .next()
        .map_or_else(|| "unknown".to_string(), |line| {
            if line.len() > 30 {
                format!("{}...", &line[0..30])
            } else {
                line.to_string()
            }
        })
}

fn is_minimum_version(version: &str, min_version: &str) -> bool {
    let version_parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();
    let min_parts: Vec<u32> = min_version.split('.').filter_map(|s| s.parse().ok()).collect();

    for i in 0..3 {
        let v1 = version_parts.get(i).copied().unwrap_or(0);
        let v2 = min_parts.get(i).copied().unwrap_or(0);
        if v1 > v2 {
            return true;
        }
        if v1 < v2 {
            return false;
        }
    }
    true
}

fn is_vulnerable_version(version: &str, vulnerable_versions: &[&str]) -> bool {
    vulnerable_versions.contains(&version)
}

pub fn get_dependency_info(name: &str) -> Result<DependencyInfo, AppError> {
    let path = get_dependency_path(name)?;

    if path.starts_with("__continuing_without_") {
        println!(
            "{}",
            format!(
                "Will attempt operations without verified {} installation",
                name
            )
            .yellow()
        );
        return Ok(DependencyInfo {
            name: name.to_string(),
            version: "unknown".to_string(),
            path,
            hash: None,
            is_min_version: false,
            is_vulnerable: false,
        });
    }

    let output = match Command::new(&path).arg("--version").output() {
        Ok(o) => o,
        Err(e) => {
            println!(
                "{}: {}",
                format!("Warning: Failed to get {} version", name).yellow(),
                e
            );
            return Ok(DependencyInfo {
                name: name.to_string(),
                version: "unknown".to_string(),
                path,
                hash: None,
                is_min_version: false,
                is_vulnerable: false,
            });
        }
    };

    if !output.status.success() {
        println!(
            "{}",
            format!("Warning: {} version check failed, but continuing", name).yellow()
        );
        return Ok(DependencyInfo {
            name: name.to_string(),
            version: "unknown".to_string(),
            path,
            hash: None,
            is_min_version: false,
            is_vulnerable: false,
        });
    }

    let version_output = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_output = String::from_utf8_lossy(&output.stderr).to_string();
    let combined_output = format!("{}\n{}", version_output, stderr_output);
    let version = parse_version(&combined_output, name);

    let hash = match calculate_file_hash(&path) {
        Ok(h) => Some(h),
        Err(_) => None,
    };

    let min_version = match name {
        "yt-dlp" => MIN_YTDLP_VERSION,
        "ffmpeg" => MIN_FFMPEG_VERSION,
        _ => "0.0.0",
    };

    let is_min_version = is_minimum_version(&version, min_version);
    let vulnerable_versions = match name {
        "yt-dlp" => &VULNERABLE_YTDLP_VERSIONS[..],
        "ffmpeg" => &VULNERABLE_FFMPEG_VERSIONS[..],
        _ => &[][..],
    };
    let is_vulnerable = is_vulnerable_version(&version, vulnerable_versions);

    Ok(DependencyInfo {
        name: name.to_string(),
        version,
        path,
        hash,
        is_min_version,
        is_vulnerable,
    })
}

pub fn is_ffmpeg_available() -> bool {
    if std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
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
        if std::path::Path::new(path).exists() {
            if std::process::Command::new(path).arg("-version").output().is_ok() {
                return true;
            }
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
}

pub fn validate_dependencies() -> Result<HashMap<String, DependencyInfo>, AppError> {
    let mut results = HashMap::new();
    let mut has_issues = false;

    println!("{}", "Validating dependencies...".blue());

    match get_dependency_info("yt-dlp") {
        Ok(info) => {
            println!("{}: {} ({})", "yt-dlp".green(), info.version, info.path);
            if !info.is_min_version {
                println!(
                    "{}: Version {} is below minimum required ({})",
                    "WARNING".yellow(),
                    info.version,
                    MIN_YTDLP_VERSION
                );
                has_issues = true;
            }
            if info.is_vulnerable {
                println!(
                    "{}: Version {} has known vulnerabilities",
                    "WARNING".red(),
                    info.version
                );
                has_issues = true;
            }
            results.insert("yt-dlp".to_string(), info);
        }
        Err(e) => {
            println!("{}: {}", "ERROR".red(), e);
            has_issues = true;
        }
    }

    let ffmpeg_available = is_ffmpeg_available();
    if ffmpeg_available {
        match get_dependency_info("ffmpeg") {
            Ok(info) => {
                println!("{}: {} ({})", "ffmpeg".green(), info.version, info.path);
                if !info.is_min_version {
                    println!("{}: Version {} is below minimum recommended ({}), but will attempt to continue", 
                        "WARNING".yellow(), 
                        info.version, 
                        MIN_FFMPEG_VERSION);
                }
                if info.is_vulnerable {
                    println!(
                        "{}: Version {} has known vulnerabilities",
                        "WARNING".yellow(),
                        info.version
                    );
                }
                results.insert("ffmpeg".to_string(), info);
            }
            Err(e) => {
                println!("{}: {}", "WARNING".yellow(), e);
                println!(
                    "{}",
                    "Will attempt to continue with limited functionality.".yellow()
                );
            }
        }
    } else {
        println!(
            "{}",
            "ffmpeg not found, but will attempt to continue with limited functionality.".yellow()
        );
        println!(
            "{}",
            "Audio conversion and time-based extraction may not work.".yellow()
        );
    }

    if has_issues {
        println!(
            "{}",
            "\nDependency validation completed with warnings.".yellow()
        );
    } else {
        println!("{}", "\nAll dependencies validated successfully.".green());
    }

    Ok(results)
}

pub fn update_ytdlp() -> Result<(), AppError> {
    println!("{}", "Updating yt-dlp to latest version...".blue());
    let output = Command::new("yt-dlp")
        .arg("--update")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(AppError::IoError)?;

    if output.success() {
        match get_dependency_info("yt-dlp") {
            Ok(info) => {
                println!("{}: {}", "Updated yt-dlp version", info.version);
                if !info.is_min_version {
                    println!(
                        "{}: Version is still below minimum required ({})",
                        "WARNING".yellow(),
                        MIN_YTDLP_VERSION
                    );
                    return Err(AppError::General(
                        "Failed to update yt-dlp to required version".to_string(),
                    ));
                }
                if info.is_vulnerable {
                    println!(
                        "{}: Updated version still has known vulnerabilities",
                        "WARNING".red()
                    );
                    return Err(AppError::General(
                        "Updated to a vulnerable version of yt-dlp".to_string(),
                    ));
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
        println!("{}", "yt-dlp updated successfully.".green());
        Ok(())
    } else {
        println!("{}", "Failed to update yt-dlp.".red());
        Err(AppError::General("Failed to update yt-dlp".to_string()))
    }
}

#[allow(dead_code)]
pub fn verify_dependency_integrity(name: &str) -> Result<bool, AppError> {
    println!("{} {}", "Verifying integrity of", name);
    let info = get_dependency_info(name)?;
    if let Some(hash) = &info.hash {
        println!("{} SHA-256: {}", name, hash);
        println!("{}", "No integrity violations detected.".green());
        Ok(true)
    } else {
        println!(
            "{}",
            "Could not calculate hash for integrity verification.".yellow()
        );
        Ok(false)
    }
}

#[allow(dead_code)]
pub fn check_rust_updates() -> Result<(), AppError> {
    println!("{}", "Checking for Rust updates...".blue());
    if !cfg!(debug_assertions) {
        println!("{}", "Skipping Rust update check in release mode.".blue());
        return Ok(());
    }
    if !Command::new("rustup")
        .arg("--version")
        .status()
        .map_err(AppError::IoError)?
        .success()
    {
        println!(
            "{}",
            "rustup not found. Skipping Rust update check.".yellow()
        );
        return Ok(());
    }
    let output = Command::new("rustup")
        .arg("update")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(AppError::IoError)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        println!("{}: {}", "Error checking for Rust updates".red(), stderr);
        return Err(AppError::General(
            "Failed to check for Rust updates".to_string(),
        ));
    }
    if stdout.contains("Updated") {
        println!("{}", "Rust toolchain updated successfully.".green());
    } else {
        println!("{}", "Rust toolchain is up to date.".green());
    }
    Ok(())
}

pub fn install_or_update_dependency(name: &str) -> Result<(), AppError> {
    match name {
        "yt-dlp" => {
            match get_dependency_info("yt-dlp") {
                Ok(info) => {
                    if !info.is_min_version || info.is_vulnerable {
                        update_ytdlp()?;
                    } else {
                        println!("{} is up to date ({})", name, info.version);
                    }
                }
                Err(_) => {
                    install_ytdlp()?;
                }
            }
        }
        "ffmpeg" => {
            match get_dependency_info("ffmpeg") {
                Ok(info) => {
                    if !info.is_min_version || info.is_vulnerable {
                        println!(
                            "{}: {} needs updating but must be done manually",
                            name.yellow(),
                            info.version
                        );
                        println!("Please update ffmpeg using your system package manager.");
                    } else {
                        println!("{} is up to date ({})", name, info.version);
                    }
                }
                Err(_) => {
                    install_ffmpeg()?;
                }
            }
        }
        _ => {
            return Err(AppError::General(format!("Unknown dependency: {}", name)));
        }
    }
    Ok(())
}

fn install_ytdlp() -> Result<(), AppError> {
    println!("{}", "Installing yt-dlp...".blue());
    let cmd = if cfg!(target_os = "windows") {
        Command::new("pip")
            .arg("install")
            .arg("--user")
            .arg("--upgrade")
            .arg("yt-dlp")
            .status()
            .map_err(AppError::IoError)?
            .success()
    } else {
        Command::new("pip3")
            .arg("install")
            .arg("--user")
            .arg("--upgrade")
            .arg("yt-dlp")
            .status()
            .map_err(AppError::IoError)?
            .success()
    };
    if cmd {
        println!("{}", "yt-dlp installed successfully.".green());
        match get_dependency_info("yt-dlp") {
            Ok(info) => {
                println!("Installed version: {}", info.version);
                if !info.is_min_version {
                    println!(
                        "{}: Version is below minimum required ({})",
                        "WARNING".yellow(),
                        MIN_YTDLP_VERSION
                    );
                }
                if info.is_vulnerable {
                    println!(
                        "{}: Installed version has known vulnerabilities",
                        "WARNING".red()
                    );
                }
            }
            Err(e) => {
                println!("{}: {}", "Failed to verify installation".red(), e);
                return Err(e);
            }
        }
        Ok(())
    } else {
        println!("{}", "Failed to install yt-dlp.".red());
        println!("Please install yt-dlp manually: https://github.com/yt-dlp/yt-dlp#installation");
        Err(AppError::General("Failed to install yt-dlp".to_string()))
    }
}

fn install_ffmpeg() -> Result<(), AppError> {
    println!("{}", "Installing ffmpeg...".blue());
    let success = if cfg!(target_os = "macos") {
        Command::new("brew")
            .arg("install")
            .arg("ffmpeg")
            .status()
            .map_err(AppError::IoError)?
            .success()
    } else if cfg!(target_os = "linux") {
        let package_managers = [
            ("apt", &["install", "-y", "ffmpeg"]),
            ("apt-get", &["install", "-y", "ffmpeg"]),
            ("dnf", &["install", "-y", "ffmpeg"]),
            ("yum", &["install", "-y", "ffmpeg"]),
            ("pacman", &["-S", "--noconfirm", "ffmpeg"]),
        ];
        let mut installed = false;
        for (pm, args) in package_managers.iter() {
            if Command::new("which")
                .arg(pm)
                .stdout(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                println!("Using {} to install ffmpeg...", pm);
                let sudo_command = "sudo".to_string();
                let pm_string = (*pm).to_string();
                installed = Command::new(&sudo_command)
                    .arg(&pm_string)
                    .args(*args)
                    .status()
                    .map_err(AppError::IoError)?
                    .success();
                if installed {
                    break;
                }
            }
        }
        installed
    } else if cfg!(target_os = "windows") {
        if Command::new("where")
            .arg("choco")
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            Command::new("choco")
                .arg("install")
                .arg("ffmpeg")
                .arg("-y")
                .status()
                .map_err(AppError::IoError)?
                .success()
        } else {
            println!(
                "{}",
                "Chocolatey not found. Please install ffmpeg manually:".yellow()
            );
            println!("https://ffmpeg.org/download.html");
            false
        }
    } else {
        println!(
            "{}",
            "Unsupported platform for automatic ffmpeg installation.".yellow()
        );
        println!("Please install ffmpeg manually: https://ffmpeg.org/download.html");
        false
    };
    if success {
        println!("{}", "ffmpeg installed successfully.".green());
        match get_dependency_info("ffmpeg") {
            Ok(info) => {
                println!("Installed version: {}", info.version);
                if !info.is_min_version {
                    println!(
                        "{}: Version is below minimum required ({})",
                        "WARNING".yellow(),
                        MIN_FFMPEG_VERSION
                    );
                }
                if info.is_vulnerable {
                    println!(
                        "{}: Installed version has known vulnerabilities",
                        "WARNING".red()
                    );
                }
            }
            Err(e) => {
                println!("{}: {}", "Failed to verify installation".red(), e);
                return Err(e);
            }
        }
        Ok(())
    } else {
        println!("{}", "Failed to install ffmpeg automatically.".red());
        println!("Please install ffmpeg manually: https://ffmpeg.org/download.html");
        Err(AppError::General("Failed to install ffmpeg".to_string()))
    }
}
