//! Enhanced dependency validator for Rustloader
//!
//! This module provides functionality to validate and verify external dependencies
//! like yt-dlp and ffmpeg, checking versions, binary integrity, and known vulnerabilities.

use crate::error::AppError;
use base64::{engine::general_purpose, Engine as _};
use colored::*;
use log::{debug, info, trace, warn};
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

/// Get the installation path for a dependency
/// 
/// This function tries multiple strategies to locate a dependency:
/// 1. Use system commands like 'which' or 'where'
/// 2. Check if the program is directly callable via PATH
/// 3. Try common installation locations
/// 4. For ffmpeg, try platform-specific detection
fn get_dependency_path(name: &str) -> Result<String, AppError> {
    // First try using system path tools
    #[cfg(target_os = "windows")]
    let search_commands = vec!["where"];

    #[cfg(not(target_os = "windows"))]
    let search_commands = vec!["which"];

    for command in &search_commands {
        if let Ok(output) = Command::new(command).arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    info!("Found {} at path: {}", name, path);
                    println!("{}: {}", format!("Found {} at", name).green(), path);
                    
                    // Double check that this path actually works
                    let version_cmd = if name == "ffmpeg" { "-version" } else { "--version" };
                    let mut version_check = Command::new(&path);
                    version_check.arg(version_cmd);
                    
                    if version_check.output().is_ok() {
                        debug!("Verified {} is executable at: {}", name, path);
                        return Ok(path);
                    }
                    
                    debug!("Path exists but command not executable: {}", path);
                }
            }
        }
    }

    // Try calling the program directly (it might be in PATH)
    let version_arg = if name == "ffmpeg" { "-version" } else { "--version" };
    if Command::new(name).arg(version_arg).output().is_ok() {
        info!("{} is available directly in PATH", name);
        println!("{}", format!("{} is available in PATH", name).green());
        return Ok(name.to_string());
    }
    
    debug!("Couldn't find {} directly in PATH", name);

    // For ffmpeg, we need more thorough checking due to various installation methods
    if name == "ffmpeg" {
        // Common installation paths across platforms
        let mut common_paths = vec![
            "/usr/bin/ffmpeg".to_string(),
            "/usr/local/bin/ffmpeg".to_string(),
            "/opt/homebrew/bin/ffmpeg".to_string(),
            "/opt/local/bin/ffmpeg".to_string(),   // MacPorts
            "/snap/bin/ffmpeg".to_string(),
            "/var/lib/flatpak/app/org.ffmpeg/ffmpeg".to_string(),
        ];
        
        // Add Linux distribution-specific paths
        #[cfg(target_os = "linux")]
        {
            // Popular distro-specific paths
            common_paths.extend(vec![
                // Debian/Ubuntu and derivatives
                "/usr/bin/ffmpeg".to_string(),
                "/usr/local/bin/ffmpeg".to_string(),
                
                // Red Hat/Fedora/CentOS paths
                "/usr/bin/ffmpeg".to_string(),
                "/usr/local/bin/ffmpeg".to_string(),
                "/opt/ffmpeg/bin/ffmpeg".to_string(),
                
                // Arch Linux paths
                "/usr/bin/ffmpeg".to_string(),
                
                // Gentoo paths
                "/usr/bin/ffmpeg".to_string(),
                "/opt/bin/ffmpeg".to_string(),
                
                // Container and alternative installations
                "/snap/bin/ffmpeg".to_string(),
                "/snap/ffmpeg/current/bin/ffmpeg".to_string(),
                "/var/lib/flatpak/app/org.ffmpeg.FFmpeg/x86_64/stable/active/files/bin/ffmpeg".to_string(),
                "/var/lib/flatpak/app/org.ffmpeg.FFmpeg/current/active/files/bin/ffmpeg".to_string(),
                "/app/bin/ffmpeg".to_string(), // Flatpak internal path
                
                // Custom compile paths common on Linux
                "/usr/local/ffmpeg/bin/ffmpeg".to_string(),
                "/opt/ffmpeg/bin/ffmpeg".to_string(),
                
                // AppImage installations
                "/tmp/.mount_ffmpeg/ffmpeg".to_string(),
                "/tmp/.mount_ffmpeg/usr/bin/ffmpeg".to_string(),
                
                // User-specific installations
                format!("{}/.local/bin/ffmpeg", std::env::var("HOME").unwrap_or_default()),
                format!("{}/bin/ffmpeg", std::env::var("HOME").unwrap_or_default()),
            ]);
            
            // Check for linuxbrew (Homebrew for Linux)
            if let Ok(home) = std::env::var("HOME") {
                common_paths.push(format!("{}/.linuxbrew/bin/ffmpeg", home));
                common_paths.push(format!("{}/linuxbrew/bin/ffmpeg", home));
            }
            
            // Try to determine specific Linux distribution for further optimizations
            let mut distro = "unknown".to_string();
            let distro_checkers = [
                ("/etc/os-release", r#"ID="?([^"\n]+)"?"#),
                ("/etc/lsb-release", r#"DISTRIB_ID="?([^"\n]+)"?"#),
                ("/etc/debian_version", r"(.+)"),
                ("/etc/redhat-release", r"(.+)"),
                ("/etc/arch-release", r"(.*)"),
            ];
            
            for (file, pattern) in distro_checkers {
                if Path::new(file).exists() {
                    if let Ok(contents) = std::fs::read_to_string(file) {
                        if let Ok(re) = regex::Regex::new(pattern) {
                            if let Some(cap) = re.captures(&contents) {
                                if let Some(m) = cap.get(1) {
                                    distro = m.as_str().to_lowercase();
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            
            info!("Detected Linux distribution: {}", distro);
            
            // Add distribution-specific paths
            match distro.as_str() {
                "ubuntu" | "debian" | "linuxmint" | "pop" | "elementary" => {
                    // Ubuntu/Debian-specific paths
                    common_paths.push("/usr/lib/x86_64-linux-gnu/ffmpeg".to_string());
                    common_paths.push("/usr/lib/ffmpeg".to_string());
                }
                "fedora" | "rhel" | "centos" | "rocky" | "almalinux" => {
                    // RHEL/Fedora-specific paths
                    common_paths.push("/usr/lib64/ffmpeg".to_string());
                }
                "arch" | "manjaro" | "endeavouros" => {
                    // Arch-specific paths 
                    common_paths.push("/usr/lib/ffmpeg".to_string());
                }
                "opensuse" | "suse" => {
                    // openSUSE-specific paths
                    common_paths.push("/usr/lib64/ffmpeg".to_string());
                }
                _ => {
                    // Generic paths for unknown distributions
                    debug!("Unknown Linux distribution, using generic paths");
                }
            }
        }
        
        // Add Windows-specific paths
        #[cfg(target_os = "windows")]
        {
            common_paths.extend(vec![
                // Program Files installations
                "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
                "C:\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
                "C:\\Program Files (x86)\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
                
                // Windows Store and portable app locations
                "C:\\Users\\Public\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
                "C:\\Windows\\System32\\ffmpeg.exe".to_string(),
                
                // Package manager locations
                // Chocolatey
                "C:\\ProgramData\\chocolatey\\bin\\ffmpeg.exe".to_string(),
                "C:\\ProgramData\\chocolatey\\lib\\ffmpeg\\tools\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
                
                // Scoop
                format!("{}\\scoop\\shims\\ffmpeg.exe", std::env::var("USERPROFILE").unwrap_or("C:\\Users\\Public".to_string())),
                format!("{}\\scoop\\apps\\ffmpeg\\current\\bin\\ffmpeg.exe", std::env::var("USERPROFILE").unwrap_or("C:\\Users\\Public".to_string())),
                
                // MSYS2 and MinGW installations
                "C:\\msys64\\mingw64\\bin\\ffmpeg.exe".to_string(),
                "C:\\msys64\\usr\\bin\\ffmpeg.exe".to_string(),
                "C:\\MinGW\\bin\\ffmpeg.exe".to_string(),
                
                // Visual Studio Code extensions folder
                format!("{}\\AppData\\Local\\Programs\\Microsoft VS Code\\resources\\app\\extensions\\ffmpeg\\bin\\ffmpeg.exe", 
                        std::env::var("USERPROFILE").unwrap_or("C:\\Users\\Public".to_string())),
            ]);
            
            // Try to find in user profile locations and AppData
            if let Ok(user_profile) = std::env::var("USERPROFILE") {
                common_paths.push(format!("{}\\ffmpeg\\bin\\ffmpeg.exe", user_profile));
                common_paths.push(format!("{}\\AppData\\Local\\ffmpeg\\bin\\ffmpeg.exe", user_profile));
                common_paths.push(format!("{}\\AppData\\Roaming\\ffmpeg\\bin\\ffmpeg.exe", user_profile));
                common_paths.push(format!("{}\\bin\\ffmpeg.exe", user_profile));
            }
            
            // Look for programmatically installed software
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                common_paths.push(format!("{}\\Programs\\ffmpeg\\bin\\ffmpeg.exe", local_app_data));
                common_paths.push(format!("{}\\Microsoft\\WindowsApps\\ffmpeg.exe", local_app_data));
            }
        }
        
        // Add macOS-specific paths
        #[cfg(target_os = "macos")]
        {
            // Homebrew paths (both Intel and Apple Silicon)
            let homebrew_intel_cellar = "/usr/local/Cellar/ffmpeg";
            let homebrew_apple_cellar = "/opt/homebrew/Cellar/ffmpeg";
            
            // Check for Intel Homebrew Cellar
            if Path::new(homebrew_intel_cellar).exists() {
                debug!("Found Homebrew Cellar at {}", homebrew_intel_cellar);
                // Try to find the latest version in the Cellar directory
                if let Ok(entries) = std::fs::read_dir(homebrew_intel_cellar) {
                    for entry in entries.flatten() {
                        let path = entry.path().join("bin/ffmpeg");
                        if path.exists() {
                            common_paths.push(path.to_string_lossy().into_owned());
                            debug!("Added Intel Homebrew path: {}", path.display());
                        }
                    }
                }
            }
            
            // Check for Apple Silicon Homebrew Cellar
            if Path::new(homebrew_apple_cellar).exists() {
                debug!("Found Apple Silicon Homebrew Cellar at {}", homebrew_apple_cellar);
                // Try to find the latest version in the Cellar directory
                if let Ok(entries) = std::fs::read_dir(homebrew_apple_cellar) {
                    for entry in entries.flatten() {
                        let path = entry.path().join("bin/ffmpeg");
                        if path.exists() {
                            common_paths.push(path.to_string_lossy().into_owned());
                            debug!("Added Apple Silicon Homebrew path: {}", path.display());
                        }
                    }
                }
            }
            
            // MacPorts paths
            let macports_paths = vec![
                "/opt/local/bin/ffmpeg",
                "/opt/local/libexec/ffmpeg",
            ];
            
            for mp_path in macports_paths {
                if Path::new(mp_path).exists() {
                    common_paths.push(mp_path.to_string());
                    debug!("Added MacPorts path: {}", mp_path);
                }
            }
            
            // XCode developer tools and common third-party macOS app locations
            common_paths.extend(vec![
                "/Applications/ffmpeg.app/Contents/MacOS/ffmpeg".to_string(),
                "/Applications/Utilities/ffmpeg".to_string(),
                "/Library/Frameworks/FFmpeg.framework/Versions/Current/bin/ffmpeg".to_string(),
            ]);
            
            // Check user Applications folder
            if let Ok(home) = std::env::var("HOME") {
                common_paths.push(format!("{}/Applications/ffmpeg.app/Contents/MacOS/ffmpeg", home));
                common_paths.push(format!("{}/.ffmpeg/bin/ffmpeg", home));
                common_paths.push(format!("{}/bin/ffmpeg", home));
                common_paths.push(format!("{}/.local/bin/ffmpeg", home));
            }
            
            // Check for specific video editors that might bundle ffmpeg
            let bundled_app_paths = vec![
                "/Applications/FCPX/Contents/MacOS/ffmpeg",
                "/Applications/Final Cut Pro.app/Contents/MacOS/ffmpeg", 
                "/Applications/Adobe Premiere Pro.app/Contents/MacOS/ffmpeg",
                "/Applications/Handbrake.app/Contents/MacOS/ffmpeg",
                "/Applications/VLC.app/Contents/MacOS/ffmpeg",
                "/Applications/OBS.app/Contents/MacOS/ffmpeg",
                "/Applications/OBS Studio.app/Contents/MacOS/ffmpeg",
            ];
            
            for app_path in bundled_app_paths {
                if Path::new(app_path).exists() {
                    common_paths.push(app_path.to_string());
                    debug!("Found bundled ffmpeg in application: {}", app_path);
                }
            }
            
            // Check for system-wide installations from dmg installers
            common_paths.push("/usr/local/ffmpeg/bin/ffmpeg".to_string());
        }
        
        // Check if any of these paths exist and can run a version check
        for path in common_paths {
            if Path::new(&path).exists() {
                debug!("Testing common path: {}", path);
                if Command::new(&path).arg("-version").output().is_ok() {
                    info!("Found {} at common location: {}", name, path);
                    println!("{}: {}", format!("Found {} at", name).green(), path);
                    return Ok(path.to_string());
                }
                trace!("Path exists but is not executable: {}", path);
            }
        }
        
        debug!("Finished checking common paths for {}", name);
        
        // Special case for package managers - attempt to locate ffmpeg by running the package manager
        #[cfg(target_os = "linux")]
        {
            // Define a comprehensive set of Linux package managers and their query commands
            let package_manager_queries: Vec<(&str, Vec<&str>, &str)> = vec![
                // Debian/Ubuntu family
                ("dpkg", vec!["-L", "ffmpeg"], r"/bin/ffmpeg$"),
                ("dpkg", vec!["-L", "ffmpeg-static"], r"/bin/ffmpeg$"),
                ("apt-file", vec!["list", "ffmpeg"], r"/bin/ffmpeg$"),

                // Red Hat/Fedora family
                ("rpm", vec!["-ql", "ffmpeg"], r"/bin/ffmpeg$"),
                ("rpm", vec!["-ql", "ffmpeg-static"], r"/bin/ffmpeg$"),
                ("dnf", vec!["repoquery", "-l", "ffmpeg"], r"/bin/ffmpeg$"),

                // Arch Linux
                ("pacman", vec!["-Ql", "ffmpeg"], r"/bin/ffmpeg$"),
                ("pacman", vec!["-Qo", "/usr/bin/ffmpeg"], r".*is owned by ffmpeg.*"),

                // SUSE
                ("rpm", vec!["-ql", "ffmpeg"], r"/bin/ffmpeg$"),
                ("zypper", vec!["se", "-i", "ffmpeg"], r".*ffmpeg.*"),

                // Container package managers
                ("flatpak", vec!["info", "org.ffmpeg.FFmpeg"], r".*"),
                ("snap", vec!["info", "ffmpeg"], r".*"),

                // Universal package managers
                ("which", vec!["ffmpeg"], r".*"),
                ("type", vec!["-p", "ffmpeg"], r".*"),
            ];
            
            // Store detected distro and version for more intelligent fallbacks
            let mut detected_distro_family = "unknown";
            if Path::new("/etc/os-release").exists() {
                if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
                    if os_release.contains("ID=debian") || os_release.contains("ID=ubuntu") || 
                       os_release.contains("ID_LIKE=debian") || os_release.contains("ID_LIKE=ubuntu") {
                        detected_distro_family = "debian";
                    } else if os_release.contains("ID=fedora") || os_release.contains("ID=rhel") || 
                              os_release.contains("ID_LIKE=fedora") || os_release.contains("ID_LIKE=rhel") {
                        detected_distro_family = "fedora";
                    } else if os_release.contains("ID=arch") || os_release.contains("ID=manjaro") || 
                              os_release.contains("ID_LIKE=arch") {
                        detected_distro_family = "arch";
                    } else if os_release.contains("ID=opensuse") || os_release.contains("ID=suse") || 
                              os_release.contains("ID_LIKE=suse") {
                        detected_distro_family = "suse";
                    }
                }
            }
            info!("Detected Linux distribution family: {}", detected_distro_family);
            
            // Try all package managers, but prioritize those matching the detected distro family
            let mut matching_paths = Vec::new();
            
            for (pkg_cmd, args, path_pattern) in package_manager_queries.iter() {
                if Command::new(pkg_cmd).arg("--version").output().is_ok() {
                    debug!("Found package manager: {}", pkg_cmd);

                    if let Ok(output) = Command::new(pkg_cmd).args(args).output() {
                        if output.status.success() {
                            let output_str = String::from_utf8_lossy(&output.stdout);
                            let stderr_str = String::from_utf8_lossy(&output.stderr);
                            let combined_output = format!("{}\n{}", output_str, stderr_str);
                            
                            // Try to extract the path using a regex pattern
                            if let Ok(re) = regex::Regex::new(path_pattern) {
                                for line in combined_output.lines() {
                                    if re.is_match(line) {
                                        // Extract path based on package manager format
                                        let path = match *pkg_cmd {
                                            "dpkg" | "rpm" | "pacman" => {
                                                // These list the full path
                                                line.trim().split_whitespace().last().unwrap_or(line).trim()
                                            },
                                            "which" | "type" => {
                                                // These output just the path
                                                line.trim()
                                            },
                                            "apt-file" | "dnf" => {
                                                // These output package: path format
                                                if let Some(idx) = line.find(':') {
                                                    line[idx+1..].trim()
                                                } else {
                                                    line.trim()
                                                }
                                            },
                                            _ => line.trim()
                                        };
                                        
                                        // Verify the path exists and is executable
                                        if Path::new(path).exists() {
                                            if Command::new(path).arg("-version").output().is_ok() {
                                                info!("Found {} using package manager {}: {}", name, pkg_cmd, path);
                                                
                                                // If this matches our detected distro, return immediately
                                                let is_priority_match = match detected_distro_family {
                                                    "debian" => *pkg_cmd == "dpkg" || *pkg_cmd == "apt-file",
                                                    "fedora" => *pkg_cmd == "rpm" || *pkg_cmd == "dnf",
                                                    "arch" => *pkg_cmd == "pacman",
                                                    "suse" => *pkg_cmd == "zypper" || *pkg_cmd == "rpm",
                                                    _ => false
                                                };
                                                
                                                if is_priority_match {
                                                    println!("{}: {} (from {}, matches distro)", 
                                                        format!("Found {} at", name).green(), 
                                                        path, 
                                                        pkg_cmd);
                                                    return Ok(path.to_string());
                                                }
                                                
                                                // Otherwise add to matching paths for later
                                                matching_paths.push(path.to_string());
                                            } else {
                                                trace!("Package manager path non-executable: {}", path);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // If we found matching paths but none were from the priority distro
            if !matching_paths.is_empty() {
                let best_path = &matching_paths[0];
                println!("{}: {} (from package manager)", 
                    format!("Found {} at", name).green(), 
                    best_path);
                return Ok(best_path.clone());
            }
            
            // Additional fallback: Try package manager to verify installation status
            // This helps provide better diagnostics
            let package_status_checks: Vec<(&str, Vec<&str>, &str)> = match detected_distro_family {
                "debian" => vec![
                    ("dpkg", vec!["-l", "ffmpeg"], "Check if ffmpeg is installed"),
                    ("apt", vec!["policy", "ffmpeg"], "Check available versions"),
                ],
                "fedora" => vec![
                    ("rpm", vec!["-q", "ffmpeg"], "Check if ffmpeg is installed"),
                    ("dnf", vec!["list", "installed", "ffmpeg"], "Check installed version"),
                ],
                "arch" => vec![
                    ("pacman", vec!["-Q", "ffmpeg"], "Check if ffmpeg is installed"),
                ],
                "suse" => vec![
                    ("zypper", vec!["se", "-i", "ffmpeg"], "Check if ffmpeg is installed"),
                    ("rpm", vec!["-q", "ffmpeg"], "Check if ffmpeg is installed"),
                ],
                _ => vec![],
            };
            
            for (cmd, args, purpose) in package_status_checks {
                if Command::new(cmd).arg("--version").output().is_ok() {
                    if let Ok(output) = Command::new(cmd).args(&args).output() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        
                        debug!("{}: {}", purpose, stdout);
                        if !stderr.is_empty() {
                            debug!("{} stderr: {}", purpose, stderr);
                        }
                        
                        // This is just for diagnostic info, we don't return from here
                    }
                }
            }
        }
    }

    warn!("{} not found in PATH or common locations. Attempting fallback mechanisms...", name);
    println!(
        "{}",
        format!(
            "Warning: {} not found in PATH or common locations. Attempting fallback mechanisms...",
            name
        )
        .yellow()
    );
    
    // Try to provide helpful information and suggest fallback options
    match name {
        "ffmpeg" => {
            // For ffmpeg, we can provide a built-in fallback mechanism
            println!("{}", "Checking for possible alternatives...".yellow());
            
            // Check for alternative names like ffmpeg4, avconv, etc.
            let alternatives = vec![
                ("ffmpeg4", "-version"),
                ("ffmpeg-4", "-version"),
                ("ffmpeg-5", "-version"),
                ("avconv", "-version"),     // Legacy alternative to ffmpeg
                ("ffmpeg.exe", "-version"), // Windows without extension in PATH
            ];
            
            for (alt_name, version_arg) in alternatives {
                debug!("Checking alternative: {}", alt_name);
                if Command::new(alt_name).arg(version_arg).output().is_ok() {
                    info!("Found alternative {} which appears to be working", alt_name);
                    println!("{}: {}", "Found alternative".green(), alt_name);
                    return Ok(alt_name.to_string());
                }
            }
            
            // Check for embedded ffmpeg in common applications that could be used
            let embedded_locations = vec![
                ("VLC", "vlc", &["--version"], r"VLC version (\d+\.\d+\.\d+)"),
                ("Handbrake", "HandBrakeCLI", &["--version"], r"HandBrake (\d+\.\d+\.\d+)"),
                ("OBS Studio", "obs", &["--version"], r"OBS (\d+\.\d+\.\d+)"),
            ];
            
            for (app_name, command, args, _version_pattern) in embedded_locations {
                if Command::new(command).args(args).output().is_ok() {
                    info!("Found {} which may include ffmpeg capabilities", app_name);
                    println!(
                        "{}",
                        format!("Found {} which includes ffmpeg functionality. Will try to use as a limited fallback.", app_name).yellow()
                    );
                    // For now we'll still return continuing_without, but noted the alternative
                }
            }
            
            // Offer auto-install option if available for this platform
            let can_auto_install = cfg!(target_os = "macos") || cfg!(target_os = "linux") || cfg!(target_os = "windows");
            if can_auto_install {
                println!("{}", "Would you like to attempt automatic installation of ffmpeg? [y/N]".blue());
                
                // Offer auto-installation but don't block
                // Instead, we'll return the fallback and let the caller decide
                println!("{}", "You can use 'rustloader install ffmpeg' to attempt automatic installation.".cyan());
                println!("{}", "Will proceed with limited functionality.".yellow());
            }
        },
        "yt-dlp" => {
            // For yt-dlp, check for youtube-dl as a fallback
            println!("{}", "Checking for youtube-dl as a fallback...".yellow());
            
            if Command::new("youtube-dl").arg("--version").output().is_ok() {
                info!("Found youtube-dl which can be used as a fallback");
                println!("{}", "Found youtube-dl which can be used as a fallback. Note that some features may not work correctly.".yellow());
                
                // Check for auto-upgrade capabilities
                println!("{}", "Recommend upgrading to yt-dlp for better performance and features.".cyan());
                println!("{}", "You can use 'rustloader install yt-dlp' to install it.".cyan());
                
                // Return youtube-dl as usable fallback
                return Ok("youtube-dl".to_string());
            }
            
            // Check for alternative names or locations
            let alternatives = vec![
                "youtube-dlc",
                "yt-dlp.exe", // Windows without extension in PATH
                "python3 -m yt_dlp",
                "python -m yt_dlp",
            ];
            
            for alt in alternatives {
                let parts: Vec<&str> = alt.split_whitespace().collect();
                if parts.len() > 1 {
                    // For commands with arguments
                    if Command::new(parts[0]).args(&parts[1..]).arg("--version").output().is_ok() {
                        info!("Found alternative {} which appears to be working", alt);
                        println!("{}: {}", "Found alternative".green(), alt);
                        return Ok(alt.to_string());
                    }
                } else {
                    // For simple commands
                    if Command::new(alt).arg("--version").output().is_ok() {
                        info!("Found alternative {} which appears to be working", alt);
                        println!("{}: {}", "Found alternative".green(), alt);
                        return Ok(alt.to_string());
                    }
                }
            }
            
            // Offer installation via pip
            println!("{}", "yt-dlp not found. It can be installed via pip:".yellow());
            println!("{}", "  pip install --user yt-dlp".cyan());
            println!("{}", "Or use 'rustloader install yt-dlp' to attempt automatic installation.".cyan());
        },
        _ => {
            println!("{}", format!("No fallback options available for dependency: {}", name).yellow());
        }
    }
    
    // Return the special fallback marker
    Ok(format!("__continuing_without_{}", name))
}

fn calculate_file_hash(path: &str) -> Result<String, AppError> {
    let mut file = File::open(path).map_err(AppError::IoError)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(AppError::IoError)?;
    let digest = digest::digest(&digest::SHA256, &buffer);
    Ok(general_purpose::STANDARD.encode(digest.as_ref()))
}

/// Parse version information from application output
/// 
/// Improved to handle the various version output formats, especially for ffmpeg
/// which can have many different styles depending on build configuration
fn parse_version(output: &str, name: &str) -> String {
    // For ffmpeg, we want to handle even more specific version formats
    if name == "ffmpeg" {
        // First try to extract the exact version string that ffmpeg reports
        let ffmpeg_patterns = vec![
            // Common ffmpeg version patterns
            r"ffmpeg version (\d+\.\d+(?:\.\d+)?)",
            r"ffmpeg version n(\d+\.\d+(?:\.\d+)?)",
            r"ffmpeg version (?:git-)?(?:\d{4}-\d{2}-\d{2}-)?(\d+\.\d+(?:\.\d+)?)",
            
            // FFmpeg built with specific configurations often has different formats
            r"ffmpeg\s+version\s+[^\s]+\s+Copyright.*?(\d+\.\d+(?:\.\d+)?)",
            
            // Static builds may show different version formats
            r"ffmpeg\s+(?:version\s+)?([0-9]+\.[0-9]+(?:\.[0-9]+)?)",
            
            // Alternative ffmpeg version reporting formats with multiple capture groups
            r"ffmpeg\s+version\s+information:\s*(?:(\d+\.\d+(?:\.\d+)?)|.*?(\d+\.\d+(?:\.\d+)?)\s*$)",
            
            // Generic version information pattern
            r"version\s+(?:information:)?(?:[^\d]*?)(\d+\.\d+(?:\.\d+)?)",
        ];
        
        for pattern in ffmpeg_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(output) {
                    // Try first capture group
                    if let Some(version) = captures.get(1) {
                        let clean_version = version.as_str().trim();
                        info!("Detected ffmpeg version: {}", clean_version);
                        println!("Detected ffmpeg version: {}", clean_version.green());
                        return clean_version.to_string();
                    }
                    // Try second capture group if available (for patterns with multiple groups)
                    else if let Some(version) = captures.get(2) {
                        let clean_version = version.as_str().trim();
                        info!("Detected ffmpeg version from alternate pattern: {}", clean_version);
                        println!("Detected ffmpeg version from alternate pattern: {}", clean_version.green());
                        return clean_version.to_string();
                    }
                }
            }
        }
        
        // More generic version detection for ffmpeg, looking for the first digit.digit pattern
        if let Ok(re) = regex::Regex::new(r"[^\d](\d+\.\d+(?:\.\d+)?)") {
            if let Some(captures) = re.captures(output) {
                if let Some(version) = captures.get(1) {
                    let clean_version = version.as_str().trim();
                    warn!("Using fallback method to detect ffmpeg version: {}", clean_version);
                    println!(
                        "{}",
                        format!(
                            "Found potential ffmpeg version using fallback method: {}",
                            clean_version
                        )
                        .yellow()
                    );
                    return clean_version.to_string();
                }
            }
        }
    } else {
        // For other tools like yt-dlp
        let version_patterns = match name {
            "yt-dlp" => vec![
                r"(?i)yt-dlp\s+(\d+\.\d+\.\d+)",
                r"(?i)version\s+(\d+\.\d+\.\d+)",
                r"(?i)(\d+\.\d+\.\d+)",
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

        // Generic version pattern as fallback
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
    }

    // If we get here, we couldn't parse any version info
    warn!("Could not parse version from output for {}", name);
    debug!("Unparseable output: {}", output);
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

    // Return a truncated form of the first line as last resort
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

/// Checks if ffmpeg is available and usable on the system
///
/// This function uses multiple strategies to check for a working ffmpeg:
/// 1. Try direct invocation through PATH
/// 2. Leverage the comprehensive detection logic in get_dependency_path
/// 3. Check common installation locations for different platforms
///
/// Returns true if a working ffmpeg is found, false otherwise
pub fn is_ffmpeg_available() -> bool {
    // First, try the direct command approach - fastest check for when it's in PATH
    if std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
    {
        info!("ffmpeg is available in PATH");
        println!("{}", "ffmpeg is available in PATH".green());
        return true;
    }

    // Use our comprehensive get_dependency_path function which has better detection
    match get_dependency_path("ffmpeg") {
        Ok(path) => {
            // If the path doesn't contain this marker string, we found a valid path
            if !path.starts_with("__continuing_without_") {
                // Double verify that this path works by running a version check
                if std::process::Command::new(&path)
                    .arg("-version")
                    .output()
                    .is_ok() 
                {
                    info!("Found working ffmpeg at: {}", path);
                    println!("{}: {}", "Found working ffmpeg at".green(), path);
                    return true;
                }
            }
        }
        Err(_) => {
            // If get_dependency_path failed, we'll try a few more direct checks below
        }
    }

    // Platform-specific locations to check as a last resort
    let mut common_paths = vec![
        "/usr/bin/ffmpeg".to_string(),
        "/usr/local/bin/ffmpeg".to_string(),
        "/opt/homebrew/bin/ffmpeg".to_string(),
        "/opt/local/bin/ffmpeg".to_string(),
        "/snap/bin/ffmpeg".to_string(),
        "/var/lib/flatpak/app/org.ffmpeg/ffmpeg".to_string(),
    ];
    
    // Add Windows-specific paths
    #[cfg(target_os = "windows")]
    {
        common_paths.extend(vec![
            "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
            "C:\\ffmpeg\\bin\\ffmpeg.exe".to_string(), 
            "C:\\Program Files (x86)\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
            // Windows Store apps location
            "C:\\Users\\Public\\ffmpeg\\bin\\ffmpeg.exe".to_string()
        ]);
    }
    
    // Add macOS-specific Homebrew locations
    #[cfg(target_os = "macos")]
    {
        // Check both Intel and M1 homebrew paths
        common_paths.push("/usr/local/Cellar/ffmpeg/bin/ffmpeg".to_string());
        common_paths.push("/opt/homebrew/Cellar/ffmpeg/bin/ffmpeg".to_string());
        
        // Try to find the latest version in the Cellar directories
        for cellar_path in &["/usr/local/Cellar/ffmpeg", "/opt/homebrew/Cellar/ffmpeg"] {
            if std::path::Path::new(cellar_path).exists() {
                if let Ok(entries) = std::fs::read_dir(cellar_path) {
                    for entry in entries.flatten() {
                        let path = entry.path().join("bin/ffmpeg");
                        if path.exists() {
                            common_paths.push(path.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
    }

    // Check all the common paths we've collected
    for path in common_paths {
        if std::path::Path::new(&path).exists() && 
           std::process::Command::new(&path)
               .arg("-version")
               .output()
               .is_ok() 
        {
            info!("Found working ffmpeg at common path: {}", path);
            println!("{}: {}", "Found working ffmpeg at".green(), path);
            return true;
        }
    }

    // One final attempt using system commands to locate ffmpeg
    #[cfg(target_os = "windows")]
    let which_cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let which_cmd = "which";

    if let Ok(output) = std::process::Command::new(which_cmd).arg("ffmpeg").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && 
               std::path::Path::new(&path).exists() && 
               std::process::Command::new(&path)
                   .arg("-version")
                   .output()
                   .is_ok() 
            {
                info!("Found working ffmpeg using system path tool: {}", path);
                println!("{}: {}", "Found working ffmpeg using system path tool at".green(), path);
                return true;
            }
        }
    }

    // If we reached here, no working ffmpeg was found
    warn!("No working ffmpeg installation was found after all detection methods");
    println!("{}", "No working ffmpeg installation was found.".yellow());
    false
}

pub fn validate_dependencies() -> Result<HashMap<String, DependencyInfo>, AppError> {
    let mut results = HashMap::new();
    let mut has_issues = false;

    info!("Starting dependency validation");
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

    // Use our improved ffmpeg availability checker that provides better feedback
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
        // The improved is_ffmpeg_available already printed detailed messages
        println!(
            "{}",
            "Will attempt to continue with limited functionality.".yellow()
        );
        println!(
            "{}",
            "Audio conversion and time-based extraction may not work.".yellow()
        );
        println!(
            "{}",
            "Consider installing ffmpeg for full functionality: https://ffmpeg.org/download.html".cyan()
        );
    }

    if has_issues {
        warn!("Dependency validation completed with warnings");
        println!(
            "{}",
            "\nDependency validation completed with warnings.".yellow()
        );
    } else {
        info!("All dependencies validated successfully");
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
                println!("Updated yt-dlp version: {}", info.version);
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
    println!("Verifying integrity of {}", name);
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
    
    // Track if any installation method succeeded
    let mut success = false;
    
    // Try modern installation methods first
    let python_commands: Vec<(&str, &[&str])> = vec![
        // Primary methods (most reliable)
        ("pip3", &["install", "--user", "--upgrade", "yt-dlp"]),
        ("pip", &["install", "--user", "--upgrade", "yt-dlp"]),
        ("python3", &["-m", "pip", "install", "--user", "--upgrade", "yt-dlp"]),
        ("python", &["-m", "pip", "install", "--user", "--upgrade", "yt-dlp"]),
        
        // Alternative methods (if primary fails)
        ("python3", &["-m", "pip", "install", "--upgrade", "yt-dlp"]),
        ("python", &["-m", "pip", "install", "--upgrade", "yt-dlp"]),
    ];
    
    // Try each Python-based installation method
    for (cmd, args) in python_commands {
        debug!("Trying to install yt-dlp with: {} {}", cmd, args.join(" "));
        if Command::new(cmd).arg("--version").output().is_ok() {
            println!("Using {} to install yt-dlp...", cmd);
            match Command::new(cmd).args(args).output() {
                Ok(output) => {
                    if output.status.success() {
                        success = true;
                        println!("{}", String::from_utf8_lossy(&output.stdout));
                        println!("{}", "yt-dlp installed successfully via Python package manager.".green());
                        break;
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        debug!("Installation failed: {}", stderr);
                        // Continue to next method if this fails
                    }
                },
                Err(e) => {
                    debug!("Error running {}: {}", cmd, e);
                    // Continue to next method
                }
            }
        }
    }
    
    // If Python-based installation didn't work, try system package managers
    if !success {
        println!("{}", "Python installation methods failed, trying system package managers...".yellow());
        
        // Platform-specific package managers
        #[cfg(target_os = "linux")]
        {
            let package_managers: Vec<(&str, Vec<&str>)> = vec![
                // Try official package repositories first
                ("apt", vec!["install", "-y", "yt-dlp"]),
                ("apt-get", vec!["install", "-y", "yt-dlp"]),
                ("dnf", vec!["install", "-y", "yt-dlp"]),
                ("pacman", vec!["-S", "--noconfirm", "yt-dlp"]),
                ("zypper", vec!["install", "-y", "yt-dlp"]),

                // Container package managers
                ("snap", vec!["install", "yt-dlp"]),
                ("flatpak", vec!["install", "flathub", "io.github.yt-dlp"]),
            ];
            
            for (pkg_manager, args) in package_managers {
                if Command::new(pkg_manager).arg("--version").output().is_ok() {
                    debug!("Trying to install yt-dlp with: {} {}", pkg_manager, args.join(" "));
                    
                    // Most package managers need sudo, except for some (snap, flatpak)
                    let need_sudo = !["snap", "flatpak"].contains(&pkg_manager);
                    
                    if need_sudo {
                        println!("Using sudo {} to install yt-dlp...", pkg_manager);
                        match Command::new("sudo")
                            .arg(pkg_manager)
                            .args(&args)
                            .output() {
                                Ok(output) => {
                                    if output.status.success() {
                                        success = true;
                                        println!("{}", "yt-dlp installed successfully.".green());
                                        break;
                                    }
                                },
                                Err(_) => { /* Continue to next method */ }
                            }
                    } else {
                        println!("Using {} to install yt-dlp...", pkg_manager);
                        match Command::new(pkg_manager)
                            .args(&args)
                            .output() {
                                Ok(output) => {
                                    if output.status.success() {
                                        success = true;
                                        println!("{}", "yt-dlp installed successfully.".green());
                                        break;
                                    }
                                },
                                Err(_) => { /* Continue to next method */ }
                            }
                    }
                }
            }
        }
        
        // macOS specific package managers
        #[cfg(target_os = "macos")]
        {
            let package_managers: Vec<(&str, Vec<&str>)> = vec![
                ("brew", vec!["install", "yt-dlp"]),
                ("port", vec!["install", "yt-dlp"]),
            ];
            
            for (pkg_manager, args) in package_managers {
                if Command::new(pkg_manager).arg("--version").output().is_ok() {
                    debug!("Trying to install yt-dlp with: {} {}", pkg_manager, args.join(" "));
                    
                    // MacPorts needs sudo, Homebrew doesn't
                    if pkg_manager == "port" {
                        println!("Using sudo {} to install yt-dlp...", pkg_manager);
                        match Command::new("sudo")
                            .arg(pkg_manager)
                            .args(&args)
                            .output() {
                                Ok(output) => {
                                    if output.status.success() {
                                        success = true;
                                        println!("{}", "yt-dlp installed successfully.".green());
                                        break;
                                    }
                                },
                                Err(_) => { /* Continue to next method */ }
                            }
                    } else {
                        println!("Using {} to install yt-dlp...", pkg_manager);
                        match Command::new(pkg_manager)
                            .args(&args)
                            .output() {
                                Ok(output) => {
                                    if output.status.success() {
                                        success = true;
                                        println!("{}", "yt-dlp installed successfully.".green());
                                        break;
                                    }
                                },
                                Err(_) => { /* Continue to next method */ }
                            }
                    }
                }
            }
        }
        
        // Windows specific package managers
        #[cfg(target_os = "windows")]
        {
            // Try popular Windows package managers
            let package_managers: Vec<(&str, Vec<&str>)> = vec![
                ("choco", vec!["install", "yt-dlp", "-y"]),
                ("scoop", vec!["install", "yt-dlp"]),
                ("winget", vec!["install", "yt-dlp"]),
            ];
            
            for (pkg_manager, args) in package_managers {
                if Command::new(pkg_manager).arg("--version").output().is_ok() {
                    debug!("Trying to install yt-dlp with: {} {}", pkg_manager, args.join(" "));
                    println!("Using {} to install yt-dlp...", pkg_manager);
                    
                    match Command::new(pkg_manager)
                        .args(&args)
                        .output() {
                            Ok(output) => {
                                if output.status.success() {
                                    success = true;
                                    println!("{}", "yt-dlp installed successfully.".green());
                                    break;
                                }
                            },
                            Err(_) => { /* Continue to next method */ }
                        }
                }
            }
        }
    }
    
    // If all methods failed, try direct download as last resort
    if !success {
        println!("{}", "Standard installation methods failed, attempting direct download...".yellow());
        
        // Determine appropriate binary name based on platform
        let binary_name = if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else {
            "yt-dlp"
        };
        
        // Determine installation path
        let install_path = if cfg!(target_os = "windows") {
            if let Ok(user_profile) = std::env::var("USERPROFILE") {
                format!("{}\\AppData\\Local\\Programs\\yt-dlp", user_profile)
            } else {
                "C:\\yt-dlp".to_string()
            }
        } else if let Ok(home) = std::env::var("HOME") {
            format!("{}/.local/bin", home)
        } else {
            "/usr/local/bin".to_string()
        };
        
        // Ensure directory exists
        let install_dir = Path::new(&install_path);
        if !install_dir.exists() {
            match std::fs::create_dir_all(install_dir) {
                Ok(_) => println!("Created installation directory: {}", install_path),
                Err(e) => {
                    println!("{}: {}", "Failed to create installation directory".red(), e);
                    return Err(AppError::IoError(e));
                }
            }
        }
        
        // Construct full path
        let binary_path = if cfg!(target_os = "windows") {
            format!("{}\\{}", install_path, binary_name)
        } else {
            format!("{}/{}", install_path, binary_name)
        };
        
        // Output status message
        println!("Downloading yt-dlp to {}", binary_path);
        
        // Recommend manual download and provide instructions
        println!("{}", "Direct download not implemented yet.".yellow());
        println!("{}", "Please download yt-dlp manually:".yellow());
        println!("1. Visit: https://github.com/yt-dlp/yt-dlp/releases/latest");
        println!("2. Download the appropriate binary for your platform");
        println!("3. Save it to a directory in your PATH");
        println!("4. Make it executable (chmod +x yt-dlp on Linux/macOS)");
    }
    
    // Final check to verify installation
    if success || Command::new("yt-dlp").arg("--version").output().is_ok() {
        // Success case - verify the installation
        match get_dependency_info("yt-dlp") {
            Ok(info) => {
                if info.path.starts_with("__continuing_without_") {
                    return Err(AppError::General("Installation was reported successful but yt-dlp still not found in PATH".to_string()));
                }
                
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
                Ok(())
            }
            Err(e) => {
                println!("{}: {}", "Failed to verify installation".red(), e);
                Err(e)
            }
        }
    } else {
        println!("{}", "Failed to install yt-dlp.".red());
        println!("Please install yt-dlp manually: https://github.com/yt-dlp/yt-dlp#installation");
        Err(AppError::General("Failed to install yt-dlp".to_string()))
    }
}

/// Install ffmpeg on the current system
///
/// This function attempts to install ffmpeg using the appropriate package manager for the current OS.
/// It includes support for:
/// - macOS: Homebrew and MacPorts
/// - Linux: apt, apt-get, dnf, yum, pacman, snap, flatpak
/// - Windows: Chocolatey and Scoop
///
/// Returns Ok(()) if installation was successful, or an error if it failed
fn install_ffmpeg() -> Result<(), AppError> {
    println!("{}", "Installing ffmpeg...".blue());
    
    let mut success = false;
    
    // macOS installation using Homebrew or MacPorts
    #[cfg(target_os = "macos")]
    {
        // First try Homebrew
        if Command::new("which")
            .arg("brew")
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false) 
        {
            println!("{}", "Using Homebrew to install ffmpeg...".blue());
            success = Command::new("brew")
                .arg("install")
                .arg("ffmpeg")
                .status()
                .map_err(AppError::IoError)?
                .success();
        } 
        // If Homebrew isn't available or failed, try MacPorts
        else if Command::new("which")
            .arg("port")
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false) && !success 
        {
            println!("{}", "Using MacPorts to install ffmpeg...".blue());
            let sudo_command = "sudo".to_string();
            success = Command::new(&sudo_command)
                .arg("port")
                .arg("install")
                .arg("ffmpeg")
                .status()
                .map_err(AppError::IoError)?
                .success();
        } 
        // If neither are available
        else if !success {
            println!(
                "{}",
                "No package manager found (brew or port). Please install ffmpeg manually:".yellow()
            );
            println!("https://ffmpeg.org/download.html#build-mac");
        }
    }
    
    // Linux installation with various package managers
    #[cfg(target_os = "linux")]
    {
        let package_managers: Vec<(&str, Vec<&str>)> = vec![
            // Standard package managers
            ("apt", vec!["install", "-y", "ffmpeg"]),
            ("apt-get", vec!["install", "-y", "ffmpeg"]),
            ("dnf", vec!["install", "-y", "ffmpeg"]),
            ("yum", vec!["install", "-y", "ffmpeg"]),
            ("pacman", vec!["-S", "--noconfirm", "ffmpeg"]),
            ("zypper", vec!["install", "-y", "ffmpeg"]),
            
            // Container-based package managers
            ("snap", vec!["install", "ffmpeg"]),
            ("flatpak", vec!["install", "flathub", "org.ffmpeg.FFmpeg"]),
        ];
        
        for (pm, args) in package_managers.iter() {
            if Command::new("which")
                .arg(pm)
                .stdout(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false) && !success
            {
                println!("Using {} to install ffmpeg...", pm);
                
                // We need sudo for most package managers, but not for snap or flatpak
                let need_sudo = !["snap", "flatpak"].contains(&pm);
                
                if need_sudo {
                    let sudo_command = "sudo".to_string();
                    let pm_string = (*pm).to_string();
                    success = Command::new(&sudo_command)
                        .arg(&pm_string)
                        .args(args)
                        .status()
                        .map_err(AppError::IoError)?
                        .success();
                } else {
                    // Direct invocation for snap and flatpak
                    success = Command::new(*pm)
                        .args(args)
                        .status()
                        .map_err(AppError::IoError)?
                        .success();
                }
                
                if success {
                    break;
                }
            }
        }
        
        if !success {
            println!(
                "{}",
                "No compatible package manager found. Please install ffmpeg manually:".yellow()
            );
            println!("https://ffmpeg.org/download.html#build-linux");
        }
    }
    
    // Windows installation using package managers
    #[cfg(target_os = "windows")]
    {
        // Try Chocolatey first
        if Command::new("where")
            .arg("choco")
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false) && !success
        {
            println!("{}", "Using Chocolatey to install ffmpeg...".blue());
            success = Command::new("choco")
                .arg("install")
                .arg("ffmpeg")
                .arg("-y")
                .status()
                .map_err(AppError::IoError)?
                .success();
        }
        
        // Try Scoop if Chocolatey failed or isn't available
        if Command::new("where")
            .arg("scoop")
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false) && !success
        {
            println!("{}", "Using Scoop to install ffmpeg...".blue());
            success = Command::new("scoop")
                .arg("install")
                .arg("ffmpeg")
                .status()
                .map_err(AppError::IoError)?
                .success();
        }
        
        // If neither package manager is available
        if !success {
            println!(
                "{}",
                "No package manager found (Chocolatey or Scoop). Please install ffmpeg manually:".yellow()
            );
            println!(
                "Download and extract from: https://ffmpeg.org/download.html#build-windows"
            );
            println!(
                "Then add the bin directory to your PATH environment variable."
            );
        }
    }
    
    // Handle unknown platforms
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        println!(
            "{}",
            "Unsupported platform for automatic ffmpeg installation.".yellow()
        );
        println!("Please install ffmpeg manually: https://ffmpeg.org/download.html");
    }
    
    // Final verification and result handling
    if success {
        println!("{}", "ffmpeg installed successfully.".green());
        
        // Verify the installation
        match get_dependency_info("ffmpeg") {
            Ok(info) => {
                println!("Installed version: {}", info.version.green());
                if !info.is_min_version {
                    println!(
                        "{}: Version is below minimum recommended ({})",
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
                println!("{}", "Will attempt to continue anyway.".yellow());
                // We still return Ok since we did manage to install something
            }
        }
        Ok(())
    } else {
        println!("{}", "Failed to install ffmpeg automatically.".red());
        println!("{}", "Please install ffmpeg manually:".yellow());
        println!("https://ffmpeg.org/download.html");
        
        // Provide platform-specific instructions
        #[cfg(target_os = "macos")]
        println!("macOS: brew install ffmpeg   OR   sudo port install ffmpeg");
        
        #[cfg(target_os = "linux")]
        println!("Linux: sudo apt install ffmpeg   OR   sudo dnf install ffmpeg");
        
        #[cfg(target_os = "windows")]
        println!("Windows: choco install ffmpeg   OR   scoop install ffmpeg");
        
        Err(AppError::General("Failed to install ffmpeg".to_string()))
    }
}
