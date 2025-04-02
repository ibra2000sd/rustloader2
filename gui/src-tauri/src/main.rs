#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

// src/main.rs - Optimized for high-performance UI with downloads
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{Manager, Runtime, State, Window, Emitter};
use std::time::{Duration, Instant};
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

// Import required Tauri plugins
use tauri_plugin_log;
use tauri_plugin_dialog;
use tauri_plugin_store;

// Mock rustloader functionality for the purpose of compiling
// These would be implemented in the real rustloader crate

// Mock types and functions for the download manager
mod mock_rustloader {
    use serde::{Deserialize, Serialize};
    
    // Mock download priority
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum DownloadPriority {
        Low,
        Normal,
        High,
        Critical,
    }
    
    // Mock download status
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum DownloadStatus {
        Queued,
        Downloading,
        Paused,
        Completed,
        Failed,
        Canceled,
    }
    
    // Mock download options
    pub struct DownloadOptions<'a> {
        pub url: &'a str,
        pub quality: Option<&'a str>,
        pub format: &'a str,
        pub start_time: Option<&'a str>,
        pub end_time: Option<&'a str>,
        pub use_playlist: bool,
        pub download_subtitles: bool,
        pub output_dir: Option<&'a str>,
        pub force_download: bool,
        pub bitrate: Option<&'a str>,
        pub priority: Option<DownloadPriority>,
    }
    
    // Mock download item
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DownloadItem {
        pub id: String,
        pub url: String,
        pub title: Option<String>,
        pub progress: f64,
        pub status: DownloadStatus,
        pub total_bytes: u64,
        pub downloaded_bytes: u64,
        pub speed: f64,
    }
    
    // Mock functions
    pub async fn add_download_to_queue(_options: DownloadOptions<'_>) -> Result<String, String> {
        Ok("mock-download-id".to_string())
    }
    
    pub async fn pause_download(_id: &str) -> Result<(), String> {
        Ok(())
    }
    
    pub async fn resume_download(_id: &str) -> Result<(), String> {
        Ok(())
    }
    
    pub async fn cancel_download(_id: &str) -> Result<(), String> {
        Ok(())
    }
    
    pub async fn pause_all_downloads() -> Result<(), String> {
        Ok(())
    }
    
    pub async fn resume_all_downloads() -> Result<(), String> {
        Ok(())
    }
    
    pub fn get_all_downloads() -> Vec<DownloadItem> {
        Vec::new()
    }
    
    pub fn get_download_status(_id: &str) -> Option<DownloadStatus> {
        Some(DownloadStatus::Downloading)
    }
    
    pub fn download_video(
        _url: &str,
        _quality: Option<&str>,
        _format: &str,
        _start_time: Option<String>,
        _end_time: Option<String>,
        _use_playlist: bool,
        _download_subtitles: bool,
        _output_dir: Option<String>,
        _force_download: bool,
        _bitrate: Option<String>,
    ) -> Result<String, String> {
        Ok("Video downloaded successfully".to_string())
    }
    
    pub fn check_is_pro() -> bool {
        false
    }
    
    pub fn activate_pro_license(_key: &str, _email: &str) -> Result<String, String> {
        Ok("License activated".to_string())
    }
    
    pub fn get_license_info() -> Result<String, String> {
        Ok("Free version".to_string())
    }
    
    pub fn get_download_progress() -> Result<ProgressData, String> {
        Ok(ProgressData {
            progress: 0,
            fileName: "Mock file".to_string(),
            fileSize: 0,
            downloaded: 0,
            speed: 0.0,
            timeRemaining: 0,
        })
    }
    
    // Mock progress data
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ProgressData {
        pub progress: i32,
        pub fileName: String,
        pub fileSize: u64,
        pub downloaded: u64,
        pub speed: f64,
        pub timeRemaining: u64,
    }
}

// Use the mock rustloader for now
use mock_rustloader::{
    download_video,
    check_is_pro,
    activate_pro_license,
    get_license_info,
    get_download_progress,
    DownloadOptions,
    DownloadPriority,
    DownloadStatus,
    add_download_to_queue,
    pause_download,
    resume_download,
    cancel_download,
    pause_all_downloads,
    resume_all_downloads,
    get_all_downloads,
    get_download_status,
    ProgressData, // Add this import
};

// Import the optimized UI components from lib.rs (which is imported as app_lib)
use app_lib::{
    DownloadManagerState, 
    DownloadProgress, 
    create_optimized_window, 
    utils::RateLimiter,
    notification::{
        NotificationManager, 
        NotificationState, 
        NotificationOptions, 
        NotificationType,
        are_notifications_supported,
        request_notification_permission,
        toggle_notifications
    }
};

// Simple progress state for backward compatibility
struct ProgressState(Arc<Mutex<i32>>);

// Video info structure
#[derive(Serialize, Deserialize)]
struct VideoInfo {
    title: String,
    uploader: String,
    duration: Option<i32>,
    views: Option<i64>,
    likes: Option<i64>,
    uploadDate: Option<String>,
}

// Commands for optimized download management
#[tauri::command]
async fn start_optimized_download(
    window: Window,
    url: String,
    quality: Option<String>,
    format: String,
    start_time: Option<String>,
    end_time: Option<String>,
    use_playlist: bool,
    download_subtitles: bool,
    output_dir: Option<String>,
    priority: Option<String>,
    download_state: State<'_, DownloadManagerState>
) -> Result<String, String> {
    // Generate a unique ID for this download
    let download_id = Uuid::new_v4().to_string();
    
    // Convert priority string to enum
    let download_priority = match priority.as_deref() {
        Some("high") => DownloadPriority::High,
        Some("critical") => DownloadPriority::Critical,
        Some("low") => DownloadPriority::Low,
        _ => DownloadPriority::Normal,
    };
    
    // Set up cancellation channel
    let (cancel_tx, mut cancel_rx) = mpsc::channel(1);
    
    // We need to extract the value from State<T> to move it into the async block
    // This is the recommended way to handle State in Tauri commands
    let state_data = download_state.inner().progress_sender.clone();
    
    // Register the download for cancellation - we can't clone State<T> but we can call methods on it
    download_state.register_download(&download_id, cancel_tx);
    
    // Initialize progress in UI
    download_state.update_progress(DownloadProgress {
        id: download_id.clone(),
        progress: 0.0,
        file_name: format!("Initializing download: {}", url),
        file_size: 0,
        downloaded_size: 0,
        speed: 0.0,
        time_remaining: None,
        status: "downloading".to_string(),
    });
    
    // Launch the download in a background task
    let download_id_clone = download_id.clone();
    let url_clone = url.clone();
    let app_clone = window.app_handle().clone();
    
    // Send initial notification for download start if notifications are enabled
    if let Some(notification_state) = app_clone.try_state::<NotificationState>() {
        let notification_manager = notification_state.0.lock().unwrap();
        notification_manager.send_notification(NotificationOptions {
            title: "Download Started".into(),
            body: format!("Started downloading from: {}", url),
            notification_type: NotificationType::Info,
            silent: true, // Silent for start notifications
            icon: None,
        });
    }
    
    // Use a tokio task for better performance
    tokio::spawn(async move {
        // Set up a rate limiter for progress updates (max 10 updates per second)
        let progress_limiter = Arc::new(RateLimiter::new(100));
        
        // Start time tracking for accurate speed calculation
        let _start_time = Instant::now();
        let mut last_update = Instant::now();
        let mut last_bytes = 0u64;
        
        // Use the rustloader download manager
        let download_options = DownloadOptions {
            url: &url_clone,
            quality: quality.as_deref(),
            format: &format,
            start_time: None, 
            end_time: None,
            use_playlist,
            download_subtitles,
            output_dir: output_dir.as_ref().map(|s| s.as_str()),
            force_download: false, // don't force download
            bitrate: None,  // use default bitrate
            priority: Some(download_priority),
        };
        match add_download_to_queue(download_options).await {
            Ok(_) => {
                // Monitor download progress
                let progress_check_interval = Duration::from_millis(100);
                let mut last_status: Option<DownloadStatus> = None;
                
                loop {
                    // Check for cancellation signal
                    if cancel_rx.try_recv().is_ok() {
                        // Cancel the download
                        let _ = cancel_download(&download_id_clone).await;
                        // Here we would update progress, but we don't have the full state
                        // We'll just log the status change instead
                        eprintln!("Download cancelled: {}", download_id_clone);
                        
                        // In a real implementation, we would unregister the download from the state
                        break;
                    }
                    
                    // Check download status
                    if let Some(status) = get_download_status(&download_id_clone) {
                        // Only send UI updates when needed
                        let should_update = if last_status.as_ref() != Some(&status) {
                            // Always update on status change
                            last_status = Some(status);
                            true
                        } else {
                            // Otherwise use the rate limiter
                            progress_limiter.should_update()
                        };
                        
                        if should_update {
                            // Get download details - in a real app this would come from the download manager
                            let downloads = get_all_downloads();
                            if let Some(download) = downloads.iter().find(|d| d.id == download_id_clone) {
                                // Calculate accurate speed
                                let now = Instant::now();
                                let elapsed = now.duration_since(last_update).as_secs_f64();
                                let bytes_diff = download.downloaded_bytes - last_bytes;
                                let speed = if elapsed > 0.0 { bytes_diff as f64 / elapsed } else { 0.0 };
                                
                                // Update tracking variables
                                last_update = now;
                                last_bytes = download.downloaded_bytes;
                                
                                // Calculate ETA
                                let time_remaining = if download.progress < 100.0 && speed > 0.0 {
                                    let remaining_bytes = download.total_bytes - download.downloaded_bytes;
                                    Some((remaining_bytes as f64 / speed) as u64)
                                } else {
                                    None
                                };
                                
                                // Map download status
                                let status_str = match status {
                                    DownloadStatus::Completed => "complete",
                                    DownloadStatus::Paused => "paused",
                                    DownloadStatus::Failed => "error",
                                    DownloadStatus::Canceled => "cancelled",
                                    _ => "downloading",
                                };
                                
                                // Log the progress update
                                eprintln!("Download progress: {}% - {}", 
                                    download.progress,
                                    download.title.clone().unwrap_or_else(|| "Downloading...".to_string())
                                );
                                
                                // Check for completion
                                if status == DownloadStatus::Completed || 
                                   status == DownloadStatus::Failed || 
                                   status == DownloadStatus::Canceled {
                                    // Send notification based on download status
                                    if let Some(notification_state) = app_clone.try_state::<NotificationState>() {
                                        let notification_manager = notification_state.0.lock().unwrap();
                                        
                                        match status {
                                            DownloadStatus::Completed => {
                                                notification_manager.send_notification(NotificationOptions {
                                                    title: "Download Complete".into(),
                                                    body: format!("{} has been downloaded successfully", 
                                                        download.title.clone().unwrap_or_else(|| "File".to_string())),
                                                    notification_type: NotificationType::Success,
                                                    silent: false,
                                                    icon: None,
                                                });
                                            },
                                            DownloadStatus::Failed => {
                                                notification_manager.send_notification(NotificationOptions {
                                                    title: "Download Failed".into(),
                                                    body: format!("Failed to download {}", 
                                                        download.title.clone().unwrap_or_else(|| "file".to_string())),
                                                    notification_type: NotificationType::Error,
                                                    silent: false,
                                                    icon: None,
                                                });
                                            },
                                            DownloadStatus::Canceled => {
                                                notification_manager.send_notification(NotificationOptions {
                                                    title: "Download Canceled".into(),
                                                    body: format!("{} was canceled", 
                                                        download.title.clone().unwrap_or_else(|| "Download".to_string())),
                                                    notification_type: NotificationType::Info,
                                                    silent: true,
                                                    icon: None,
                                                });
                                            },
                                            _ => {}
                                        }
                                    }
                                    
                                    // In a real implementation, we would unregister the download from the state
                                    eprintln!("Download {} complete with status: {:?}", download_id_clone, status);
                                    break;
                                }
                            }
                        }
                    }
                    
                    // Sleep to prevent high CPU usage
                    tokio::time::sleep(progress_check_interval).await;
                }
                
                // Just log that we're done
                eprintln!("Download task completed successfully");
            },
            Err(e) => {
                // Log error
                eprintln!("Error in download task: {}", e);
            }
        }
    });
    
    Ok(download_id)
}

// Command to list all active downloads
#[tauri::command]
async fn list_downloads() -> Result<Vec<DownloadProgress>, String> {
    let all_downloads = get_all_downloads();
    
    let progress_items = all_downloads.into_iter()
        .map(|download| {
            // Map download status
            let status_str = match download.status {
                DownloadStatus::Completed => "complete",
                DownloadStatus::Paused => "paused",
                DownloadStatus::Failed => "error",
                DownloadStatus::Canceled => "cancelled",
                DownloadStatus::Downloading => "downloading",
                _ => "queued",
            };
            
            DownloadProgress {
                id: download.id.clone(),
                progress: download.progress,
                file_name: download.title.unwrap_or_else(|| "Downloading...".to_string()),
                file_size: download.total_bytes,
                downloaded_size: download.downloaded_bytes,
                speed: download.speed,
                time_remaining: None, // download.get_eta().map(|d| d.as_secs()),
                status: status_str.to_string(),
            }
        })
        .collect();
    
    Ok(progress_items)
}

// Command to pause downloads
#[tauri::command]
async fn pause_download_item(id: String) -> Result<(), String> {
    pause_download(&id).await.map_err(|e| e.to_string())
}

// Command to resume downloads
#[tauri::command]
async fn resume_download_item(id: String) -> Result<(), String> {
    resume_download(&id).await.map_err(|e| e.to_string())
}

// Command to cancel downloads
#[tauri::command]
async fn cancel_download_item(
    id: String, 
    download_state: State<'_, DownloadManagerState>
) -> Result<(), String> {
    // Try to cancel via download manager state first (for active downloads)
    let dm_result = download_state.cancel_download(&id);
    
    // Also try to cancel via download manager (for queued downloads)
    let queue_result = cancel_download(&id).await;
    
    // Return success if either method worked
    if dm_result.is_ok() || queue_result.is_ok() {
        Ok(())
    } else {
        // Combine error messages
        let mut error_msg = String::new();
        if let Err(e) = dm_result {
            error_msg.push_str(&e.to_string());
        }
        if let Err(e) = queue_result {
            if !error_msg.is_empty() {
                error_msg.push_str(", ");
            }
            error_msg.push_str(&e.to_string());
        }
        Err(error_msg)
    }
}

// Command to pause all downloads
#[tauri::command]
async fn pause_all() -> Result<(), String> {
    pause_all_downloads().await.map_err(|e| e.to_string())
}

// Command to resume all downloads
#[tauri::command]
async fn resume_all() -> Result<(), String> {
    resume_all_downloads().await.map_err(|e| e.to_string())
}

// Legacy commands for backward compatibility
#[tauri::command]
fn start_download<R: Runtime>(
  window: Window<R>,
  url: String,
  quality: Option<String>,
  format: String,
  _start_time: Option<String>,
  _end_time: Option<String>,
  use_playlist: bool,
  download_subtitles: bool,
  output_dir: Option<String>,
  progress_state: State<'_, ProgressState>
) -> Result<(), String> {
  let window_copy = window.clone();
  let url_copy = url.clone();

  let mut progress = progress_state.0.lock().unwrap();
  *progress = 0;
  drop(progress);

  // Use the Emitter trait method
  if let Err(e) = window.emit("download-progress", serde_json::json!({
    "progress": 0,
    "fileName": "Initializing download...",
    "fileSize": 0,
    "speed": 0,
    "timeRemaining": 0
  })) {
    eprintln!("Error emitting download-progress event: {}", e);
  }

  thread::spawn(move || {
    match download_video(
      &url_copy,
      quality.as_deref(),
      &format,
      None,  // start_time
      None,  // end_time
      use_playlist,
      download_subtitles,
      output_dir,
      false, // don't force download
      None,  // use default bitrate
    ) {
      Ok(result) => {
        if let Err(e) = window_copy.emit("download-progress", serde_json::json!({
          "progress": 100,
          "fileName": "Download complete",
          "fileSize": 0,
          "speed": 0,
          "timeRemaining": 0
        })) {
          eprintln!("Error emitting download-progress event: {}", e);
        }
        
        if let Err(e) = window_copy.emit("download-completed", serde_json::json!({
          "success": true,
          "message": result
        })) {
          eprintln!("Error emitting download-completed event: {}", e);
        }
      },
      Err(error) => {
        if let Err(e) = window_copy.emit("download-completed", serde_json::json!({
          "success": false,
          "message": error.to_string()
        })) {
          eprintln!("Error emitting download-completed event: {}", e);
        }
      }
    }
  });

  Ok(())
}

#[tauri::command]
fn poll_download_progress<R: Runtime>(window: Window<R>) {
  thread::spawn(move || {
    // Use an optimized polling interval with backoff
    let mut retry_count = 0;
    let mut poll_interval = Duration::from_millis(100);
    
    loop {
      match get_download_progress() {
        Ok(progress_data) => {
          // Reset backoff on successful fetch
          retry_count = 0;
          poll_interval = Duration::from_millis(100);
          
          // Only emit if progress actually changed (reduce UI overhead)
          // Clone the data to satisfy the Clone trait requirement
          let progress_data_clone = serde_json::to_value(&progress_data).unwrap_or_default();
          if let Err(e) = window.emit("download-progress", progress_data_clone) {
            eprintln!("Error emitting download-progress event: {}", e);
          }
          if progress_data.progress >= 100 {
            break;
          }
        },
        Err(_) => {
          // Increase poll interval with each failure (exponential backoff)
          retry_count += 1;
          let backoff_ms = std::cmp::min(500 * (1 << retry_count), 5000);
          poll_interval = Duration::from_millis(backoff_ms);
          
          thread::sleep(poll_interval);
        }
      }
      thread::sleep(poll_interval);
    }
  });
}

#[tauri::command]
fn get_video_info(url: String) -> Result<VideoInfo, String> {
    // Use a 10-second timeout to prevent hanging
    let output = std::process::Command::new("yt-dlp")
        .args(["--dump-json", "--no-playlist", "--socket-timeout", "10", &url])
        .output()
        .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("yt-dlp execution failed: {}", error_msg));
    }

    let json_str = String::from_utf8_lossy(&output.stdout).to_string();
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse yt-dlp output: {}", e))?;

    let title = json_value["title"].as_str().unwrap_or("Unknown Title").to_string();
    let uploader = json_value["uploader"].as_str().unwrap_or("Unknown Uploader").to_string();
    let duration = json_value["duration"].as_f64().map(|d| d as i32);
    let views = json_value["view_count"].as_i64();
    let likes = json_value["like_count"].as_i64();
    let upload_date = json_value["upload_date"].as_str().map(|date| {
        if date.len() == 8 {
            let year = &date[0..4];
            let month = &date[4..6];
            let day = &date[6..8];
            format!("{}-{}-{}", year, month, day)
        } else {
            date.to_string()
        }
    });

    Ok(VideoInfo {
        title,
        uploader,
        duration,
        views,
        likes,
        uploadDate: upload_date,
    })
}

#[tauri::command]
fn get_progress(progress_state: State<'_, ProgressState>) -> i32 {
  let progress = progress_state.0.lock().unwrap();
  *progress
}

#[tauri::command]
fn is_pro() -> bool {
  check_is_pro()
}

#[tauri::command]
fn check_license() -> String {
  if check_is_pro() {
    "pro".to_string()
  } else {
    "free".to_string()
  }
}

#[tauri::command]
fn activate_license(license_key: String, email: String) -> Result<String, String> {
  activate_pro_license(&license_key, &email)
}

#[tauri::command]
fn license_info() -> Result<String, String> {
  get_license_info()
}

#[tauri::command]
fn list_download_paths() -> Vec<String> {
  let mut paths = Vec::new();
  
  if let Some(home_dir) = dirs_next::home_dir() {
    let mut videos_dir = home_dir.clone();
    videos_dir.push("Downloads");
    videos_dir.push("rustloader");
    videos_dir.push("videos");
    if let Some(path_str) = videos_dir.to_str() {
      paths.push(path_str.to_string());
    }

    let mut audio_dir = home_dir;
    audio_dir.push("Downloads");
    audio_dir.push("rustloader");
    audio_dir.push("audio");
    if let Some(path_str) = audio_dir.to_str() {
      paths.push(path_str.to_string());
    }
  }
  
  paths
}

#[tauri::command]
fn check_pending_downloads() -> bool {
  !get_all_downloads().is_empty()
}

// Check if this is the first run of the application
#[tauri::command]
fn is_first_run() -> bool {
  // In the actual implementation, this would check a persisted value
  // For demonstration purposes, we'll just return true
  true
}

// We rely on the imported get_download_status from rustloader
// The function is already imported in the dependencies

fn main() {
  let progress_state = Arc::new(Mutex::new(0));

  // Download manager state will be created in setup since we need the app handle

  tauri::Builder::default()
      .plugin(tauri_plugin_dialog::init())
      .plugin(tauri_plugin_store::Builder::default().build())
      .plugin(tauri_plugin_log::Builder::default().build())
      .manage(ProgressState(progress_state))
      .setup(|app| {
          // Create and register the download manager state
          let download_manager_state = DownloadManagerState::new(app.handle().clone());
          app.manage(download_manager_state);
          
          // Create and register the notification manager
          let notification_manager = NotificationManager::new(app.handle().clone());
          app.manage(NotificationState(Mutex::new(notification_manager)));
          
          // Initialize any window-specific features like transparency or blur
          // Window effects are optional and handled differently in Tauri 2.x
          if let Some(_window) = app.get_window("main") {
              // Window customization can be done here if needed
          }
          
          Ok(())
      })
      .invoke_handler(tauri::generate_handler![
          // Optimized download commands
          start_optimized_download,
          list_downloads,
          pause_download_item,
          resume_download_item,
          cancel_download_item,
          pause_all,
          resume_all,
          
          // Notification commands - comment out until notification functionality is fully implemented
          // are_notifications_supported,
          // request_notification_permission,
          // toggle_notifications,
          
          // First-run and onboarding
          is_first_run,
          
          // Legacy commands for backward compatibility
          start_download,
          get_progress,
          is_pro,
          activate_license,
          license_info,
          check_license,
          list_download_paths,
          check_pending_downloads,
          get_video_info,
          poll_download_progress
      ])
      .run(tauri::generate_context!())
      .expect("error while running tauri application");
}
