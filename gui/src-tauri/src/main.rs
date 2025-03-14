#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex};
use tauri::State;
use rustloader::{
  download_video,
  check_is_pro,
  activate_pro_license,
  get_license_info
};

// State to track download progress
struct ProgressState(Arc<Mutex<i32>>);

// The start_download command that the frontend now uses
#[tauri::command]
fn start_download(
  url: String,
  quality: Option<String>,
  format: String,
  start_time: Option<String>,
  end_time: Option<String>,
  use_playlist: bool,
  download_subtitles: bool,
  output_dir: Option<String>,
  progress_state: State<'_, ProgressState>
) -> Result<String, String> {
  // Reset progress to 0
  let mut progress = progress_state.0.lock().unwrap();
  *progress = 0;
  drop(progress);
  
  // Convert Option<String> to Option<&str> for quality
  let quality_ref = quality.as_deref();
  
  // Call the rustloader library function
  download_video(
      &url,
      quality_ref,
      &format,
      start_time,
      end_time,
      use_playlist,
      download_subtitles,
      output_dir
  )
}

// Command to get current download progress
#[tauri::command]
fn get_progress(progress_state: State<'_, ProgressState>) -> i32 {
  let progress = progress_state.0.lock().unwrap();
  *progress
}

// Check if Pro version is active
#[tauri::command]
fn is_pro() -> bool {
  check_is_pro()
}

// Added alias for check_license that frontend might call
#[tauri::command]
fn check_license() -> String {
  if check_is_pro() {
    "pro".to_string()
  } else {
    "free".to_string()
  }
}

// Command to activate a Pro license
#[tauri::command]
fn activate_license(license_key: String, email: String) -> Result<String, String> {
  activate_pro_license(&license_key, &email)
}

// Command to get license information
#[tauri::command]
fn license_info() -> Result<String, String> {
  get_license_info()
}

// Added stub for list_download_paths that frontend might call
#[tauri::command]
fn list_download_paths() -> Vec<String> {
  // Return some default paths based on platform
  let mut paths = Vec::new();
  
  if let Some(home_dir) = dirs_next::home_dir() {
    // Add Downloads/rustloader/videos
    let mut videos_dir = home_dir.clone();
    videos_dir.push("Downloads");
    videos_dir.push("rustloader");
    videos_dir.push("videos");
    if let Some(path_str) = videos_dir.to_str() {
      paths.push(path_str.to_string());
    }
    
    // Add Downloads/rustloader/audio
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

// Added stub for check_pending_downloads that frontend might call
#[tauri::command]
fn check_pending_downloads() -> bool {
  // For now, just always return false (no pending downloads)
  false
}

fn main() {
  // Create progress state
  let progress_state = Arc::new(Mutex::new(0));
  
  tauri::Builder::default()
      .manage(ProgressState(progress_state))
      .invoke_handler(tauri::generate_handler![
          // Original commands
          start_download,
          get_progress,
          is_pro,
          activate_license,
          license_info,
          
          // Added aliases and stubs for frontend compatibility
          check_license,
          list_download_paths,
          check_pending_downloads
      ])
      .run(tauri::generate_context!())
      .expect("error while running tauri application");
}