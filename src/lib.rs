// src/lib.rs
// Expose Rustloader functionality as a library for the GUI

mod version;
pub use version::VERSION;

pub mod cli;
pub mod dependency_validator;
pub mod downloader;
pub mod error;
pub mod license;
pub mod security;
pub mod utils;

use crate::dependency_validator::validate_dependencies;
use crate::downloader::download_video_free;
use crate::license::{activate_license, is_pro_version, LicenseStatus}; // Removed display_license_info
use std::sync::Arc;
use std::sync::Mutex;
use tokio::runtime::Runtime;

/// Download a video using Rustloader core functionality
/// This function is designed to be called from the Tauri GUI
pub fn download_video(
    url: &str,
    quality: Option<&str>,
    format: &str,
    start_time: Option<String>,
    end_time: Option<String>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<String>,
) -> Result<String, String> {
    // Set up a runtime for async operations
    let rt = Runtime::new().map_err(|e| e.to_string())?;

    // Create a progress tracker for the UI to access
    let progress = Arc::new(Mutex::new(0));
    let progress_clone = Arc::clone(&progress);

    // Run the download operation in the runtime
    let result = rt.block_on(async {
        // Optional: Check dependencies
        match validate_dependencies() {
            Ok(_) => (),
            Err(e) => {
                return Err(format!("Dependency check failed: {}", e));
            }
        }

        // Convert Option<String> to Option<&String> for the function call
        let start_ref = start_time.as_ref();
        let end_ref = end_time.as_ref();
        let output_ref = output_dir.as_ref();

        // Set up a thread to update the progress
        let progress_updater = tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let mut p = progress_clone.lock().unwrap();
                if *p >= 100 {
                    break;
                }
                *p += 1;
                if *p > 99 {
                    *p = 99; // Stay at 99% until complete
                }
            }
        });

        // Call the download function
        let download_result = download_video_free(
            url,
            quality,
            format,
            start_ref,
            end_ref,
            use_playlist,
            download_subtitles,
            output_ref.as_deref(),
            false, // force_download
            None,  // bitrate
        )
        .await;

        // Set progress to 100% when finished
        let mut p = progress.lock().unwrap();
        *p = 100;

        // Abort the progress updater
        progress_updater.abort();

        // Convert the AppError to String for error cases
        download_result.map_err(|e| format!("{}", e))
    });

    // Convert the result
    match result {
        Ok(_) => Ok("Download completed successfully".to_string()),
        Err(e) => Err(format!("Download failed: {}", e)),
    }
}

/// Get the current download progress (0-100)
pub fn get_download_progress(progress: Arc<Mutex<i32>>) -> i32 {
    let p = progress.lock().unwrap();
    *p
}

/// Check if Pro version is active
pub fn check_is_pro() -> bool {
    is_pro_version()
}

/// Activate a Pro license
pub fn activate_pro_license(key: &str, email: &str) -> Result<String, String> {
    match activate_license(key, email) {
        Ok(LicenseStatus::Pro(_)) => Ok("License activated successfully".to_string()),
        Ok(_) => Err("Invalid license activation response".to_string()),
        Err(e) => Err(format!("License activation failed: {}", e)),
    }
}

/// Get license information
pub fn get_license_info() -> Result<String, String> {
    // Re-implement without using display_license_info
    if is_pro_version() {
        Ok("Pro license is active. Enjoy all premium features!".to_string())
    } else {
        Ok("Free version in use. Upgrade to Pro for additional features.".to_string())
    }
}
