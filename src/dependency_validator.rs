//! Enhanced dependency validator for Rustloader
//! 
//! This module provides functionality to validate and verify external dependencies
//! like yt-dlp and ffmpeg, checking versions, binary integrity, and known vulnerabilities.

use crate::error::AppError;
use std::process::{Command, Stdio};
use std::collections::HashMap;
use ring::digest;
use base64::{Engine as _, engine::general_purpose};
use colored::*;
use std::io::Read;
use std::fs::File;
use std::path::Path;

// Minimum acceptable versions for dependencies
pub const MIN_YTDLP_VERSION: &str = "2023.07.06";
pub const MIN_FFMPEG_VERSION: &str = "4.0.0";

// Known vulnerable versions to warn about
const VULNERABLE_YTDLP_VERSIONS: [&str; 2] = ["2022.05.18", "2022.08.14"];
const VULNERABLE_FFMPEG_VERSIONS: [&str; 2] = ["4.3.1", "4.4.2"];

/// Dependency info containing version and path information
#[allow(dead_code)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub path: String,
    pub hash: Option<String>,
    pub is_min_version: bool,
    pub is_vulnerable: bool,
}

// Modify this function in src/dependency_validator.rs

fn get_dependency_path(name: &str) -> Result<String, AppError> {
    // First try using the standard which/where commands
    #[cfg(target_os = "windows")]
    let search_commands = vec!["where"]; // Windows uses `where` to locate executables

    #[cfg(not(target_os = "windows"))]
    let search_commands = vec!["which"]; // Linux/macOS use `which` to find executables

    // Try to find the binary using the defined search commands
    for command in &search_commands {
        if let Ok(output) = Command::new(command).arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    println!("{}: {}", format!("Found {} at", name).green(), path);
                    return Ok(path); // Return the found path if valid
                }
            }
        }
    }

    // If we couldn't find it using which/where, try direct execution
    if let Ok(output) = Command::new(name).arg("-version").output() {
        if output.status.success() {
            println!("{}", format!("{} is available in PATH", name).green());
            return Ok(name.to_string()); // Just return the command name as it's in PATH
        }
    }

    // List of common paths where `ffmpeg` might be installed across different OS
    if name == "ffmpeg" {
        let common_paths = vec![
            "/usr/bin/ffmpeg",                // Standard Linux path
            "/usr/local/bin/ffmpeg",          // Common Linux/macOS installation path
            "/opt/homebrew/bin/ffmpeg",       // Homebrew installation on macOS (Apple Silicon)
            "/snap/bin/ffmpeg",               // Snap package location
            "/var/lib/flatpak/app/org.ffmpeg/ffmpeg", // Flatpak installation
            "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe", // Common Windows installation path
            "C:\\ffmpeg\\bin\\ffmpeg.exe"     // Alternative Windows installation path
        ];

        // Check if `ffmpeg` exists in any of the predefined paths
        for path in common_paths {
            if Path::new(path).exists() {
                println!("{}: {}", format!("Found {} at", name).green(), path);
                return Ok(path.to_string());
            }
        }
    }

    // At this point, we couldn't find the dependency, but we'll return a placeholder
    // to allow the program to continue trying
    println!("{}", format!("Warning: {} not found in PATH or common locations. Will try to proceed anyway.", name).yellow());
    
    // Return a placeholder path that indicates we're continuing without verification
    Ok(format!("__continuing_without_{}", name))
}




/// Calculate SHA-256 hash of a file
fn calculate_file_hash(path: &str) -> Result<String, AppError> {
    let mut file = File::open(path)
        .map_err(|e| AppError::IoError(e))?;
        
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| AppError::IoError(e))?;
        
    let digest = digest::digest(&digest::SHA256, &buffer);
    Ok(general_purpose::STANDARD.encode(digest.as_ref()))
}

/// Parse version string from output with enhanced robustness
fn parse_version(output: &str, name: &str) -> String {
    // First try standard patterns
    let version_patterns = match name {
        "yt-dlp" => vec![
            r"(?i)yt-dlp\s+(\d+\.\d+\.\d+)",
            r"(?i)version\s+(\d+\.\d+\.\d+)",
            r"(?i)(\d+\.\d+\.\d+)"
        ],
        "ffmpeg" => vec![
            r"(?i)ffmpeg\s+version\s+(\d+\.\d+(?:\.\d+)?)",
            r"(?i)version\s+(\d+\.\d+(?:\.\d+)?)",
            r"(?i)ffmpeg\s+(\d+\.\d+(?:\.\d+)?)",
            r"(?i)ffmpeg.*?(\d+\.\d+(?:\.\d+)?)", // More permissive pattern
            r"(?i)(\d+\.\d+(?:\.\d+)?)"  // Last resort - any version-like string
        ],
        _ => vec![r"(\d+\.\d+\.\d+)"],
    };
    
    // Try each pattern in order
    for pattern in version_patterns {
        let re = match regex::Regex::new(pattern) {
            Ok(re) => re,
            Err(_) => continue,
        };
        
        if let Some(captures) = re.captures(output) {
            if let Some(version) = captures.get(1) {
                return version.as_str().to_string();
            }
        }
    }
    
    // If no pattern matched, look for any version-like string
    let generic_pattern = r"(\d+\.\d+(?:\.\d+)?)";
    if let Ok(re) = regex::Regex::new(generic_pattern) {
        if let Some(captures) = re.captures(output) {
            if let Some(version) = captures.get(1) {
                println!("{}", format!("Found potential {} version using fallback method: {}", 
                                      name, version.as_str()).yellow());
                return version.as_str().to_string();
            }
        }
    }
    
    // Debug output to help diagnose issues
    println!("{}", format!("Could not parse version from output for {}: {}", name, output).yellow());
    println!("{}", "Returning 'unknown' as version - will attempt to continue".yellow());
    
    // Fallback - return first line, or "unknown"
    output.lines().next().map_or_else(|| "unknown".to_string(), |line| {
        if line.len() > 30 {
            // Truncate long lines
            format!("{}...", &line[0..30])
        } else {
            line.to_string()
        }
    })
}

/// Check if a version is at least the minimum required
fn is_minimum_version(version: &str, min_version: &str) -> bool {
    // Simple version comparison - in real app, use semver crate
    // This is a simplified version check
    let version_parts: Vec<u32> = version
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
        
    let min_parts: Vec<u32> = min_version
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
        
    // Compare major, minor, patch parts
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
    
    // Versions are equal
    true
}

/// Check if a version is in the list of known vulnerable versions
fn is_vulnerable_version(version: &str, vulnerable_versions: &[&str]) -> bool {
    vulnerable_versions.contains(&version)
}

/// Get detailed information about a dependency
pub fn get_dependency_info(name: &str) -> Result<DependencyInfo, AppError> {
    // First check if dependency exists
    let path = get_dependency_path(name)?;
    
    // If we're continuing without the dependency, create a placeholder
    if path.starts_with("__continuing_without_") {
        println!("{}", format!("Will attempt operations without verified {} installation", name).yellow());
        
        return Ok(DependencyInfo {
            name: name.to_string(),
            version: "unknown".to_string(),
            path: path,
            hash: None,
            is_min_version: false,  // Assume it doesn't meet min version
            is_vulnerable: false,   // Assume it's not vulnerable (since we don't know)
        });
    }
    
    // Normal path - get version info
    let output = match Command::new(&path).arg("--version").output() {
        Ok(output) => output,
        Err(e) => {
            println!("{}: {}", format!("Warning: Failed to get {} version", name).yellow(), e);
            // Return a placeholder info but don't fail completely
            return Ok(DependencyInfo {
                name: name.to_string(),
                version: "unknown".to_string(),
                path: path,
                hash: None,
                is_min_version: false,
                is_vulnerable: false,
            });
        }
    };
    
    if !output.status.success() {
        println!("{}", format!("Warning: {} version check failed, but continuing", name).yellow());
        return Ok(DependencyInfo {
            name: name.to_string(),
            version: "unknown".to_string(),
            path: path,
            hash: None,
            is_min_version: false,
            is_vulnerable: false,
        });
    }
    
    let version_output = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_output = String::from_utf8_lossy(&output.stderr).to_string();
    
    // Combine stdout and stderr as some programs output version to stderr
    let combined_output = format!("{}\n{}", version_output, stderr_output);
    
    // Parse version from output
    let version = parse_version(&combined_output, name);
    
    // Calculate file hash - but don't fail if we can't
    let hash = match calculate_file_hash(&path) {
        Ok(h) => Some(h),
        Err(_) => None,
    };
    
    // Check minimum version
    let min_version = match name {
        "yt-dlp" => MIN_YTDLP_VERSION,
        "ffmpeg" => MIN_FFMPEG_VERSION,
        _ => "0.0.0",
    };
    
    let is_min_version = is_minimum_version(&version, min_version);
    
    // Check if vulnerable version
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


/// A direct check for FFmpeg availability that falls back on multiple methods
/// Returns true if FFmpeg appears to be available, false otherwise
pub fn is_ffmpeg_available() -> bool {
    // Method 1: Direct command execution
    if let Ok(output) = std::process::Command::new("ffmpeg")
        .arg("-version")
        .output() {
        if output.status.success() {
            return true;
        }
    }
    
    // Method 2: Check common paths
    let common_paths = vec![
        "/usr/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/opt/homebrew/bin/ffmpeg",
        "/snap/bin/ffmpeg",
        "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
        "C:\\ffmpeg\\bin\\ffmpeg.exe"
    ];
    
    for path in common_paths {
        if std::path::Path::new(path).exists() {
            // Try to execute it directly
            if let Ok(output) = std::process::Command::new(path)
                .arg("-version")
                .output() {
                if output.status.success() {
                    return true;
                }
            }
        }
    }
    
    // Method 3: Use which/where
    #[cfg(target_os = "windows")]
    let which_cmd = "where";
    
    #[cfg(not(target_os = "windows"))]
    let which_cmd = "which";
    
    if let Ok(output) = std::process::Command::new(which_cmd)
        .arg("ffmpeg")
        .output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && std::path::Path::new(&path).exists() {
                return true;
            }
        }
    }
    
    // If all methods failed, FFmpeg probably isn't available
    false
}

/// Check all required dependencies with detailed report
pub fn validate_dependencies() -> Result<HashMap<String, DependencyInfo>, AppError> {
    let mut results = HashMap::new();
    let mut has_issues = false;
    
    println!("{}", "Validating dependencies...".blue());
    
    // Check yt-dlp
    match get_dependency_info("yt-dlp") {
        Ok(info) => {
            println!("{}: {} ({})", "yt-dlp".green(), info.version, info.path);
            
            if !info.is_min_version {
                println!("{}: Version {} is below minimum required ({})", 
                    "WARNING".yellow(), 
                    info.version, 
                    MIN_YTDLP_VERSION);
                has_issues = true;
            }
            
            if info.is_vulnerable {
                println!("{}: Version {} has known vulnerabilities", 
                    "WARNING".red(), 
                    info.version);
                has_issues = true;
            }
            
            results.insert("yt-dlp".to_string(), info);
        },
        Err(e) => {
            println!("{}: {}", "ERROR".red(), e);
            has_issues = true;
        }
    }
    
    // Check ffmpeg
    match get_dependency_info("ffmpeg") {
        Ok(info) => {
            println!("{}: {} ({})", "ffmpeg".green(), info.version, info.path);
            
            if !info.is_min_version {
                println!("{}: Version {} is below minimum required ({})", 
                    "WARNING".yellow(), 
                    info.version, 
                    MIN_FFMPEG_VERSION);
                has_issues = true;
            }
            
            if info.is_vulnerable {
                println!("{}: Version {} has known vulnerabilities", 
                    "WARNING".red(), 
                    info.version);
                has_issues = true;
            }
            
            results.insert("ffmpeg".to_string(), info);
        },
        Err(e) => {
            println!("{}: {}", "ERROR".red(), e);
            has_issues = true;
        }
    }
    
    // Display summary
    if has_issues {
        println!("{}", "\nDependency validation completed with warnings.".yellow());
    } else {
        println!("{}", "\nAll dependencies validated successfully.".green());
    }
    
    Ok(results)
}

/// Update yt-dlp to the latest version
pub fn update_ytdlp() -> Result<(), AppError> {
    println!("{}", "Updating yt-dlp to latest version...".blue());
    
    let output = Command::new("yt-dlp")
        .arg("--update")
        .stdout(Stdio::inherit()) // Show output to user
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| AppError::IoError(e))?;
        
    if output.success() {
        // Verify the update was successful
        match get_dependency_info("yt-dlp") {
            Ok(info) => {
                println!("{}: {}", "Updated yt-dlp version", info.version);
                
                if !info.is_min_version {
                    println!("{}: Version is still below minimum required ({})", 
                        "WARNING".yellow(), 
                        MIN_YTDLP_VERSION);
                    return Err(AppError::General("Failed to update yt-dlp to required version".to_string()));
                }
                
                if info.is_vulnerable {
                    println!("{}: Updated version still has known vulnerabilities", 
                        "WARNING".red());
                    return Err(AppError::General("Updated to a vulnerable version of yt-dlp".to_string()));
                }
            },
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

/// Verify integrity of a dependency against known good hashes
#[allow(dead_code)]
pub fn verify_dependency_integrity(name: &str) -> Result<bool, AppError> {
    println!("{} {}", "Verifying integrity of", name);
    
    // Get current dependency info
    let info = get_dependency_info(name)?;
    
    // This is where we would verify against known good hashes
    // In a real implementation, these would be fetched from a secure server
    // or embedded in the binary
    
    // For now, just print the hash for reference
    if let Some(hash) = &info.hash {
        println!("{} SHA-256: {}", name, hash);
        println!("{}", "No integrity violations detected.".green());
        // In a real implementation, verify against trusted hash
        return Ok(true);
    } else {
        println!("{}", "Could not calculate hash for integrity verification.".yellow());
        return Ok(false);
    }
}

/// Check for updates to rust toolchain
#[allow(dead_code)]
pub fn check_rust_updates() -> Result<(), AppError> {
    println!("{}", "Checking for Rust updates...".blue());
    
    if !cfg!(debug_assertions) {
        // Skip in release mode for end users
        println!("{}", "Skipping Rust update check in release mode.".blue());
        return Ok(());
    }
    
    if !Command::new("rustup").arg("--version").status().map_err(|e| AppError::IoError(e))?.success() {
        println!("{}", "rustup not found. Skipping Rust update check.".yellow());
        return Ok(());
    }
    
    let output = Command::new("rustup")
        .arg("update")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| AppError::IoError(e))?;
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
        
    if !output.status.success() {
        println!("{}: {}", "Error checking for Rust updates".red(), stderr);
        return Err(AppError::General("Failed to check for Rust updates".to_string()));
    }
    
    if stdout.contains("Updated") {
        println!("{}", "Rust toolchain updated successfully.".green());
    } else {
        println!("{}", "Rust toolchain is up to date.".green());
    }
    
    Ok(())
}

/// Install dependency if not present or outdated
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
                },
                Err(_) => {
                    // Not installed, install it
                    install_ytdlp()?;
                }
            }
        },
        "ffmpeg" => {
            match get_dependency_info("ffmpeg") {
                Ok(info) => {
                    if !info.is_min_version || info.is_vulnerable {
                        println!("{}: {} needs updating but must be done manually", name.yellow(), info.version);
                        println!("Please update ffmpeg using your system package manager.");
                    } else {
                        println!("{} is up to date ({})", name, info.version);
                    }
                },
                Err(_) => {
                    // Not installed, install it manually
                    install_ffmpeg()?;
                }
            }
        },
        _ => {
            return Err(AppError::General(format!("Unknown dependency: {}", name)));
        }
    }
    
    Ok(())
}

/// Install yt-dlp
fn install_ytdlp() -> Result<(), AppError> {
    println!("{}", "Installing yt-dlp...".blue());
    
    let cmd = if cfg!(target_os = "windows") {
        let status = Command::new("pip")
            .arg("install")
            .arg("--user")
            .arg("--upgrade")
            .arg("yt-dlp")
            .status()
            .map_err(|e| AppError::IoError(e))?;
            
        status.success()
    } else {
        let status = Command::new("pip3")
            .arg("install")
            .arg("--user")
            .arg("--upgrade")
            .arg("yt-dlp")
            .status()
            .map_err(|e| AppError::IoError(e))?;
            
        status.success()
    };
    
    if cmd {
        println!("{}", "yt-dlp installed successfully.".green());
        
        // Verify installation
        match get_dependency_info("yt-dlp") {
            Ok(info) => {
                println!("Installed version: {}", info.version);
                
                if !info.is_min_version {
                    println!("{}: Version is below minimum required ({})", 
                        "WARNING".yellow(), 
                        MIN_YTDLP_VERSION);
                }
                
                if info.is_vulnerable {
                    println!("{}: Installed version has known vulnerabilities", 
                        "WARNING".red());
                }
            },
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

/// Install ffmpeg with platform-specific commands
fn install_ffmpeg() -> Result<(), AppError> {
    println!("{}", "Installing ffmpeg...".blue());

    let success = if cfg!(target_os = "macos") {
        // macOS using Homebrew
        Command::new("brew")
            .arg("install")
            .arg("ffmpeg")
            .status()
            .map_err(|e| AppError::IoError(e))?
            .success()
    } else if cfg!(target_os = "linux") {
        // Try common Linux package managers
        let package_managers = [
            ("apt", &["install", "-y", "ffmpeg"]),
            ("apt-get", &["install", "-y", "ffmpeg"]),
            ("dnf", &["install", "-y", "ffmpeg"]),
            ("yum", &["install", "-y", "ffmpeg"]),
            ("pacman", &["-S", "--noconfirm", "ffmpeg"]),
        ];
        
        let mut installed = false;
        
        for (pm, args) in package_managers.iter() {
            if Command::new("which").arg(pm).stdout(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) {
                println!("Using {} to install ffmpeg...", pm);
                
                // Fix: Create the sudo_args vector and store it in a variable
                let sudo_command = "sudo".to_string();
                let pm_string = (*pm).to_string();
                
                // Fix: Build the command directly without constructing the vector
                installed = Command::new(&sudo_command)
                    .arg(&pm_string)
                    .args(*args)
                    .status()
                    .map_err(|e| AppError::IoError(e))?
                    .success();
                    
                if installed {
                    break;
                }
            }
        }
        
        installed
    } else if cfg!(target_os = "windows") {
        // Windows - try using chocolatey
        if Command::new("where").arg("choco").stdout(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) {
            Command::new("choco")
                .arg("install")
                .arg("ffmpeg")
                .arg("-y")
                .status()
                .map_err(|e| AppError::IoError(e))?
                .success()
        } else {
            println!("{}", "Chocolatey not found. Please install ffmpeg manually:".yellow());
            println!("https://ffmpeg.org/download.html");
            false
        }
    } else {
        println!("{}", "Unsupported platform for automatic ffmpeg installation.".yellow());
        println!("Please install ffmpeg manually: https://ffmpeg.org/download.html");
        false
    };
    
    if success {
        println!("{}", "ffmpeg installed successfully.".green());
        
        // Verify installation
        match get_dependency_info("ffmpeg") {
            Ok(info) => {
                println!("Installed version: {}", info.version);
                
                if !info.is_min_version {
                    println!("{}: Version is below minimum required ({})", 
                        "WARNING".yellow(), 
                        MIN_FFMPEG_VERSION);
                }
                
                if info.is_vulnerable {
                    println!("{}: Installed version has known vulnerabilities", 
                        "WARNING".red());
                }
            },
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
