// src/main.rs

mod cli;
mod dependency_validator;
mod downloader;
mod error;
mod license;
mod security; // Module declaration
mod utils;

use cli::build_cli;
use colored::*;
use dependency_validator::{install_or_update_dependency, validate_dependencies};
use downloader::download_video_free;
use error::AppError;
// Removed duplicate `use security;` import
use license::{activate_license, display_license_info, is_pro_version, LicenseStatus};
use rand::Rng;
use utils::check_for_updates; // Removed unnecessary braces

// Logo and version information
const VERSION: &str = "1.0.0";
// Remove static IS_PRO flag and replace with dynamic license check

// Only include the startup messages for main.rs since that's all we use here
struct StartupPromo {
    messages: Vec<String>,
}

impl StartupPromo {
    fn new() -> Self {
        Self {
            messages: vec![
                "🚀 Rustloader Pro offers 4K video downloads and 5X faster speeds! 🚀".to_string(),
                "💎 Upgrade to Rustloader Pro for AI-powered video upscaling! 💎".to_string(),
                "🎵 Enjoy high-quality 320kbps MP3 and FLAC with Rustloader Pro! 🎵".to_string(),
                "🔥 Rustloader Pro removes daily download limits! 🔥".to_string(),
            ],
        }
    }

    fn get_random_message(&self) -> &str {
        let idx = rand::thread_rng().gen_range(0..self.messages.len());
        &self.messages[idx]
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize security module
    security::init();

    // Display logo and welcome message
    print_logo();

    // Check for updates in the background
    let update_check = tokio::spawn(check_for_updates());

    // Check license status - this replaces the static IS_PRO flag
    let is_pro = is_pro_version();

    if is_pro {
        println!(
            "{}",
            "Rustloader Pro - Advanced Video Downloader"
                .bright_cyan()
                .bold()
        );
        // Display license information if in Pro mode
        if let Err(e) = display_license_info() {
            eprintln!("{}: {}", "Warning".yellow(), e);
        }
    } else {
        println!("{}", "Rustloader - Video Downloader".bright_cyan().bold());
        println!("{}", format!("Version: {} (Free)", VERSION).cyan());

        // Display a promotional message for the free version
        let promo = StartupPromo::new();
        println!("\n{}\n", promo.get_random_message().bright_yellow());
    }

    // Perform enhanced dependency validation
    println!("{}", "Performing enhanced dependency validation...".blue());

    // Modify the dependency handling section in main.rs
    // This is a partial code snippet to be inserted in the main() function

    match validate_dependencies() {
        Ok(deps) => {
            // Check if any dependencies have issues
            let mut has_issues = false;

            // Check yt-dlp status
            if let Some(info) = deps.get("yt-dlp") {
                if !info.is_min_version || info.is_vulnerable {
                    has_issues = true;
                    println!("{}", "yt-dlp needs to be updated.".yellow());
                    println!("Would you like to update yt-dlp now? (y/n):");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().eq_ignore_ascii_case("y") {
                        install_or_update_dependency("yt-dlp")?;
                    } else {
                        println!("{}", "Continuing with the current version. Some features may not work correctly.".yellow());
                    }
                }
            } else {
                has_issues = true;
                println!(
                    "{}",
                    "yt-dlp is not installed. Would you like to install it now? (y/n):".yellow()
                );
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim().eq_ignore_ascii_case("y") {
                    install_or_update_dependency("yt-dlp")?;
                } else {
                    println!(
                        "{}",
                        "yt-dlp is required. Please install it manually and try again.".red()
                    );
                    return Err(AppError::MissingDependency(
                        "yt-dlp installation declined".to_string(),
                    ));
                }
            }

            // Check ffmpeg status
            if let Some(info) = deps.get("ffmpeg") {
                if !info.is_min_version || info.is_vulnerable {
                    has_issues = true;
                    println!("{}", "ffmpeg needs to be updated.".yellow());
                    println!("Would you like to attempt to update ffmpeg now? (y/n):");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().eq_ignore_ascii_case("y") {
                        install_or_update_dependency("ffmpeg")?;
                    } else {
                        println!("{}", "Continuing with the current version. Some features may not work correctly.".yellow());
                    }
                }
            } else {
                // This is where we handle missing ffmpeg differently
                has_issues = true;
                println!(
                    "{}",
                    "ffmpeg was not detected. Some features may not work properly.".yellow()
                );
                println!("{}", "Attempting to continue without verified ffmpeg. Do you want to try to install it? (y/n):".yellow());
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim().eq_ignore_ascii_case("y") {
                    match install_or_update_dependency("ffmpeg") {
                        Ok(_) => println!("{}", "ffmpeg installed successfully.".green()),
                        Err(e) => println!(
                            "{}: {}. Will try to continue anyway.",
                            "Warning".yellow(),
                            e
                        ),
                    }
                } else {
                    println!(
                        "{}",
                        "Continuing without verified ffmpeg. Some features may not work.".yellow()
                    );
                }
            }

            if !has_issues {
                println!("{}", "All dependencies passed validation.".green());
            }
        }
        Err(e) => {
            println!("{}: {}", "Dependency validation had issues".yellow(), e);
            println!("Would you like to continue anyway? (y/n):");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                return Err(e);
            } else {
                println!(
                    "{}",
                    "Continuing with potential dependency issues...".yellow()
                );
            }
        }
    }

    // Parse command-line arguments
    let matches = build_cli().get_matches();

    // Check for license activation command
    if let Some(key) = matches.get_one::<String>("activate-license") {
        println!("{}", "License activation process started...".blue());

        // Get email for activation
        println!("Please enter your email address:");
        let mut email = String::new();
        std::io::stdin().read_line(&mut email)?;
        email = email.trim().to_string();

        // Try to activate the license
        match activate_license(key, &email)? {
            LicenseStatus::Pro(license) => {
                println!("{}", "License activated successfully!".green());
                println!("Thank you for upgrading to Rustloader Pro!");
                println!("Email: {}", license.user_email);
                println!("Activated: {}", license.activation_date);
                if let Some(exp) = license.expiration_date {
                    println!("Expires: {}", exp);
                } else {
                    println!("License Type: Perpetual (No Expiration)");
                }

                println!("\nPlease restart Rustloader to use Pro features.");
                return Ok(());
            }
            LicenseStatus::Invalid(reason) => {
                println!("{}: {}", "License activation failed".red(), reason);
                return Err(AppError::LicenseError(format!(
                    "License activation failed: {}",
                    reason
                )));
            }
            _ => {
                println!(
                    "{}",
                    "License activation failed with an unknown error".red()
                );
                return Err(AppError::LicenseError(
                    "License activation failed".to_string(),
                ));
            }
        }
    }

    // Show license information if requested
    if matches.get_flag("license-info") {
        return display_license_info();
    }

    // Extract URL and options
    let url = matches.get_one::<String>("url").unwrap();
    let quality = matches.get_one::<String>("quality").map(|q| q.as_str());
    let format = matches
        .get_one::<String>("format")
        .map(|f| f.as_str())
        .unwrap_or("mp4");
    let start_time = matches.get_one::<String>("start-time");
    let end_time = matches.get_one::<String>("end-time");
    let use_playlist = matches.get_flag("playlist");
    let download_subtitles = matches.get_flag("subtitles");
    let output_dir = matches.get_one::<String>("output-dir");

    // Only allow force download in development mode
    let force_download = if cfg!(debug_assertions) {
        let is_forced = matches.get_flag("force");
        if is_forced {
            println!(
                "{}",
                "⚠️ WARNING: Development mode force flag enabled! Daily limits bypassed. ⚠️"
                    .bright_red()
            );
            println!(
                "{}",
                "This flag should never be used in production environments.".bright_red()
            );
        }
        is_forced
    } else {
        false
    };

    let bitrate = matches.get_one::<String>("video-bitrate"); // Extract the bitrate option

    // Check for update results
    if let Ok(update_result) = update_check.await {
        if let Ok(true) = update_result {
            println!(
                "{}",
                "A new version of Rustloader is available! Visit rustloader.com to upgrade."
                    .bright_yellow()
            );
        }
    }

    // Perform video download using the free version function
    match download_video_free(
        url,
        quality,
        format,
        start_time,
        end_time,
        use_playlist,
        download_subtitles,
        output_dir,
        force_download, // Pass the force_download parameter
        bitrate,        // Pass the bitrate parameter
    )
    .await
    {
        Ok(_) => println!("{}", "Process completed successfully.".green()),
        Err(AppError::DailyLimitExceeded) => {
            eprintln!(
                "{}",
                "Daily download limit exceeded for free version."
                    .red()
                    .bold()
            );
            println!(
                "{}",
                "🚀 Upgrade to Rustloader Pro for unlimited downloads: rustloader.com/pro 🚀"
                    .bright_yellow()
            );
            return Err(AppError::DailyLimitExceeded);
        }
        Err(AppError::PremiumFeature(feature)) => {
            eprintln!("{}: {}", "Premium feature required".red().bold(), feature);
            println!(
                "{}",
                "🚀 Upgrade to Rustloader Pro to access this feature: rustloader.com/pro 🚀"
                    .bright_yellow()
            );
            return Err(AppError::PremiumFeature(feature));
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            return Err(e);
        }
    }

    Ok(())
}

fn print_logo() {
    println!(
        "\n{}",
        r"
 ____           _   _                 _           
|  _ \ _   _ __| |_| | ___   __ _  __| | ___ _ __ 
| |_) | | | / _` | | |/ _ \ / _` |/ _` |/ _ \ '__|
|  _ <| |_| \__ | |_| | (_) | (_| | (_| |  __/ |   
|_| \_\\__,_|___/\__|_|\___/ \__,_|\__,_|\___|_|   
                                                  
"
        .bright_cyan()
    );
}
