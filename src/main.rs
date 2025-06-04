// src/main.rs

mod cli;
mod dependency_validator;
mod downloader;
mod download_manager;
mod error;
mod license;
mod security;
mod utils;
mod version;

// Import modules
use cli::build_cli;
use colored::*;
use dependency_validator::{install_or_update_dependency, validate_dependencies};
use downloader::download_video_free;
use download_manager::{
    DownloadOptions, DownloadPriority, add_download_to_queue, pause_all_downloads, resume_all_downloads,
    get_download_queue, get_all_downloads, shutdown_download_manager,
};
use error::AppError;
use license::{activate_license, display_license_info, is_pro_version, LicenseStatus};
use log::{debug, error, info, warn};
use rand::Rng;
use utils::check_for_updates;

// Import env_logger for initialization
use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;

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
    // Initialize the logger with a custom format
    init_logger();
    
    // Log application startup
    info!("Rustloader starting up - version {}", VERSION);
    debug!("Debug logging enabled");
    
    // Initialize security module
    security::init();
    
    // Display logo and welcome message
    print_logo();

    // Check for updates in the background
    let update_check = tokio::spawn(check_for_updates());

    // Check license status - this replaces the static IS_PRO flag
    let is_pro = is_pro_version();
    
    if is_pro {
        info!("Starting in PRO mode");
        println!(
            "{}",
            "Rustloader Pro - Advanced Video Downloader"
                .bright_cyan()
                .bold()
        );
        // Display license information if in Pro mode
        if let Err(e) = display_license_info() {
            error!("Failed to display license information: {}", e);
            eprintln!("{}: {}", "Warning".yellow(), e);
        }
    } else {
        info!("Starting in FREE mode");
        println!("{}", "Rustloader - Video Downloader".bright_cyan().bold());
        println!("{}", format!("Version: {} (Free)", VERSION).cyan());

        // Display a promotional message for the free version
        let promo = StartupPromo::new();
        let message = promo.get_random_message();
        debug!("Selected promotional message: {}", message);
        println!("\n{}\n", message.bright_yellow());
    }

    // Perform enhanced dependency validation
    info!("Starting dependency validation");
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

    // Initialize download manager
    info!("Initializing download manager");
    let download_queue = get_download_queue().await;

    // Register a shutdown handler for the download manager
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(shutdown_download_manager());
        original_hook(panic_info);
    }));

    // Handle queue-related commands
    if let Some(queue_matches) = matches.subcommand_matches("queue") {
        // Handle queue subcommands
        if queue_matches.subcommand_matches("list").is_some() {
            // List all downloads in the queue
            let downloads = get_all_downloads();
            if downloads.is_empty() {
                println!("{}", "No downloads in queue.".blue());
            } else {
                println!("{}", "Download Queue:".bright_cyan().bold());
                println!("{}", "-".repeat(80));
                println!("{:<10} {:<20} {:<12} {:<10} {:<12} {:<15}", 
                    "ID", "Title", "Status", "Progress", "Priority", "Added");
                println!("{}", "-".repeat(80));
                
                let download_count = downloads.len();
                
                for dl in downloads {
                    let title = dl.title.unwrap_or(format!("URL: {}", dl.url));
                    let title_display = if title.len() > 18 { 
                        format!("{}...", &title[0..15]) 
                    } else { 
                        title 
                    };
                    
                    let id_short = &dl.id[0..8];
                    println!("{:<10} {:<20} {:<12} {:<10} {:<12} {:<15}",
                        id_short,
                        title_display,
                        format!("{:?}", dl.status),
                        format!("{:.1}%", dl.progress),
                        format!("{:?}", dl.priority),
                        dl.added_at.format("%Y-%m-%d %H:%M").to_string()
                    );
                }
                println!("{}", "-".repeat(80));
                println!("Total Downloads: {}", download_count);
            }
            return Ok(());
        } else if queue_matches.subcommand_matches("pause-all").is_some() {
            // Pause all active downloads
            info!("Pausing all downloads");
            match pause_all_downloads().await {
                Ok(_) => {
                    println!("{}", "All downloads paused successfully.".green());
                },
                Err(e) => {
                    println!("{}: {}", "Error pausing downloads".red(), e);
                    return Err(e);
                }
            }
            return Ok(());
        } else if queue_matches.subcommand_matches("resume-all").is_some() {
            // Resume all paused downloads
            info!("Resuming all downloads");
            match resume_all_downloads().await {
                Ok(_) => {
                    println!("{}", "All downloads resumed successfully.".green());
                },
                Err(e) => {
                    println!("{}: {}", "Error resuming downloads".red(), e);
                    return Err(e);
                }
            }
            return Ok(());
        } else if let Some(pause_matches) = queue_matches.subcommand_matches("pause") {
            // Pause a specific download
            let id = pause_matches.get_one::<String>("id").unwrap();
            info!("Pausing download: {}", id);
            
            match download_manager::pause_download(id).await {
                Ok(_) => {
                    println!("{}", format!("Download {} paused successfully.", id).green());
                },
                Err(e) => {
                    println!("{}: {}", "Error pausing download".red(), e);
                    return Err(e);
                }
            }
            return Ok(());
        } else if let Some(resume_matches) = queue_matches.subcommand_matches("resume") {
            // Resume a specific download
            let id = resume_matches.get_one::<String>("id").unwrap();
            info!("Resuming download: {}", id);
            
            match download_manager::resume_download(id).await {
                Ok(_) => {
                    println!("{}", format!("Download {} resumed successfully.", id).green());
                },
                Err(e) => {
                    println!("{}: {}", "Error resuming download".red(), e);
                    return Err(e);
                }
            }
            return Ok(());
        } else if let Some(cancel_matches) = queue_matches.subcommand_matches("cancel") {
            // Cancel a specific download
            let id = cancel_matches.get_one::<String>("id").unwrap();
            info!("Cancelling download: {}", id);
            
            match download_manager::cancel_download(id).await {
                Ok(_) => {
                    println!("{}", format!("Download {} cancelled successfully.", id).green());
                },
                Err(e) => {
                    println!("{}: {}", "Error cancelling download".red(), e);
                    return Err(e);
                }
            }
            return Ok(());
        } else if let Some(priority_matches) = queue_matches.subcommand_matches("priority") {
            // Change a download's priority
            let id = priority_matches.get_one::<String>("id").unwrap();
            let level = priority_matches.get_one::<String>("level").unwrap();
            
            let priority = match level.as_str() {
                "low" => DownloadPriority::Low,
                "normal" => DownloadPriority::Normal,
                "high" => DownloadPriority::High,
                "critical" => DownloadPriority::Critical,
                _ => DownloadPriority::Normal,
            };
            
            info!("Changing priority for download {}: {:?}", id, priority);
            
            match download_manager::set_download_priority(id, priority).await {
                Ok(_) => {
                    println!("{}", format!("Priority for download {} set to {:?}.", id, priority).green());
                },
                Err(e) => {
                    println!("{}: {}", "Error setting priority".red(), e);
                    return Err(e);
                }
            }
            return Ok(());
        } else if queue_matches.subcommand_matches("clear-completed").is_some() {
            // Clear completed downloads
            info!("Clearing completed downloads");
            
            match download_queue.remove_completed().await {
                Ok(_) => {
                    println!("{}", "Completed downloads cleared successfully.".green());
                },
                Err(e) => {
                    println!("{}: {}", "Error clearing completed downloads".red(), e);
                    return Err(e);
                }
            }
            return Ok(());
        } else if queue_matches.subcommand_matches("clear-failed").is_some() {
            // Clear failed downloads
            info!("Clearing failed downloads");
            
            match download_queue.clear_failed().await {
                Ok(_) => {
                    println!("{}", "Failed downloads cleared successfully.".green());
                },
                Err(e) => {
                    println!("{}: {}", "Error clearing failed downloads".red(), e);
                    return Err(e);
                }
            }
            return Ok(());
        }
    }
    
    // Handle download subcommand or direct URL (backward compatibility)
    let download_matches = matches.subcommand_matches("download");
    
    // Determine URL and options from either download subcommand or direct args
    let (url, quality, format, start_time, end_time, use_playlist, download_subtitles, output_dir, force_download, bitrate, use_queue, priority) =
        if let Some(dl_matches) = download_matches {
            // Get options from download subcommand
            let url = dl_matches.get_one::<String>("url").unwrap();
            let quality = dl_matches.get_one::<String>("quality").map(|q| q.as_str());
            let format = dl_matches
                .get_one::<String>("format")
                .map(|f| f.as_str())
                .unwrap_or("mp4");
            let start_time = dl_matches.get_one::<String>("start-time");
            let end_time = dl_matches.get_one::<String>("end-time");
            let use_playlist = dl_matches.get_flag("playlist");
            let download_subtitles = dl_matches.get_flag("subtitles");
            let output_dir = dl_matches.get_one::<String>("output-dir");
            
            // Only allow force download in development mode
            let force_download = if cfg!(debug_assertions) {
                dl_matches.get_flag("force")
            } else {
                false
            };
            
            let bitrate = dl_matches.get_one::<String>("video-bitrate");
            let use_queue = dl_matches.get_flag("add-to-queue");
            
            // Parse priority
            let default_priority = String::from("normal");
            let priority_str = dl_matches.get_one::<String>("priority").unwrap_or(&default_priority).as_str();
            let priority = match priority_str {
                "low" => DownloadPriority::Low,
                "normal" => DownloadPriority::Normal,
                "high" => DownloadPriority::High,
                "critical" => DownloadPriority::Critical,
                _ => DownloadPriority::Normal,
            };
            
            (url, quality, format, start_time, end_time, use_playlist, download_subtitles, output_dir, force_download, bitrate, use_queue, Some(priority))
        } else {
            // Get options from direct arguments (backward compatibility)
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
                    warn!("Development mode force flag enabled - daily limits will be bypassed");
                    debug!("Force flag should only be used in development environments");
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
            
            let bitrate = matches.get_one::<String>("video-bitrate");
            
            // Default to direct download for backward compatibility
            let use_queue = false;
            let priority = None; // Use default priority
            
            (url, quality, format, start_time, end_time, use_playlist, download_subtitles, output_dir, force_download, bitrate, use_queue, priority)
        };

    // Check for update results
    if let Ok(Ok(true)) = update_check.await {
        info!("Update check completed: new version available");
        println!(
            "{}",
            "A new version of Rustloader is available! Visit rustloader.com to upgrade."
                .bright_yellow()
        );
    } else {
        debug!("No updates available or update check failed");
    }

    // Process the download
    info!("Starting download process for URL: {}", url);
    debug!("Download parameters: quality={:?}, format={}, start_time={:?}, end_time={:?}, playlist={}, subtitles={}, output_dir={:?}, force={}, bitrate={:?}, use_queue={}, priority={:?}",
           quality, format, start_time, end_time, use_playlist, download_subtitles, output_dir, force_download, bitrate, use_queue, priority);
    
    if use_queue {
        // Add to download queue instead of downloading immediately
        info!("Adding download to queue: {}", url);
        let download_options = DownloadOptions {
            url,
            quality,
            format,
            start_time,
            end_time,
            use_playlist,
            download_subtitles,
            output_dir,
            force_download,
            bitrate,
            priority,
        };
        match add_download_to_queue(download_options).await {
            Ok(id) => {
                println!("{}", "Download added to queue successfully.".green());
                println!("Download ID: {}", id);
                println!("Use 'rustloader queue list' to view all downloads.");
            },
            Err(e) => {
                error!("Failed to add download to queue: {}", e);
                println!("{}: {}", "Error".red().bold(), e);
                return Err(e);
            }
        }
    } else {
        // Perform direct download using the free version function
        match download_video_free(
            url,
            quality,
            format,
            start_time,
            end_time,
            use_playlist,
            download_subtitles,
            output_dir,
            force_download,
            bitrate,
        )
        .await
        {
            Ok(path) => {
                info!("Download completed successfully: {}", path);
                println!("{} {}", "Process completed successfully. File saved at".green(), path);
            },
            Err(AppError::DailyLimitExceeded) => {
                error!("Daily download limit exceeded for free version");
                println!("{}", "Would you like to add to download queue instead? (y/n)".yellow());
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                
                if input.trim().eq_ignore_ascii_case("y") {
                    info!("Adding to queue instead after daily limit exceeded");
                    let download_options = DownloadOptions {
                        url,
                        quality,
                        format,
                        start_time,
                        end_time,
                        use_playlist,
                        download_subtitles,
                        output_dir,
                        force_download,
                        bitrate,
                        priority: None, // Use default priority
                    };
                    match add_download_to_queue(download_options).await {
                        Ok(id) => {
                            println!("{}", "Download added to queue successfully.".green());
                            println!("Download ID: {}", id);
                            println!("Use 'rustloader queue list' to view all downloads.");
                            println!("Download will resume when you have available download slots.");
                        },
                        Err(e) => {
                            error!("Failed to add download to queue: {}", e);
                            println!("{}: {}", "Error".red().bold(), e);
                            return Err(e);
                        }
                    }
                } else {
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
            },
            Err(AppError::PremiumFeature(feature)) => {
                error!("Premium feature required: {}", feature);
                eprintln!("{}: {}", "Premium feature required".red().bold(), feature);
                println!(
                    "{}",
                    "🚀 Upgrade to Rustloader Pro to access this feature: rustloader.com/pro 🚀"
                        .bright_yellow()
                );
                return Err(AppError::PremiumFeature(feature));
            },
            Err(e) => {
                error!("Download failed: {}", e);
                eprintln!("{}: {}", "Error".red().bold(), e);
                return Err(e);
            }
        }
    }
    
    // Make sure to cleanly shutdown the download manager
    info!("Saving download queue state before exit");
    if let Err(e) = download_queue.save_state().await {
        warn!("Failed to save download queue state: {}", e);
    }

    Ok(())
}

/// Initialize the logger with a custom format and configuration
fn init_logger() {
    // Create a custom logger builder
    let mut builder = Builder::from_default_env();
    
    // Set the default level based on debug/release mode
    if cfg!(debug_assertions) {
        builder.filter_level(LevelFilter::Debug);
    } else {
        builder.filter_level(LevelFilter::Info);
    }
    
    // Define a custom format with timestamp, level, module, and message
    builder.format(|buf, record| {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        writeln!(
            buf,
            "[{} {} {}] {}",
            timestamp,
            record.level().to_string().to_uppercase(),
            record.module_path().unwrap_or("unknown"),
            record.args()
        )
    });
    
    // Allow override through RUST_LOG environment variable
    builder.parse_env("RUST_LOG");
    
    // Initialize the logger
    builder.init();
    
    // Log library versions in debug mode
    if cfg!(debug_assertions) {
        debug!("Logger initialized with custom format");
        debug!("Running in debug mode with enhanced logging");
    }
}

fn print_logo() {
    info!("Displaying application logo");
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
