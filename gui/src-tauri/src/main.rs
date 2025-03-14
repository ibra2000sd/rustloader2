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

// Command to initiate a download
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

// Command to check if Pro version is active
#[tauri::command]
fn is_pro() -> bool {
  check_is_pro()
}

// Command to activate a Pro license
#[tauri::command]
fn activate_license(key: String, email: String) -> Result<String, String> {
  activate_pro_license(&key, &email)
}

// Command to get license information
#[tauri::command]
fn license_info() -> Result<String, String> {
  get_license_info()
}

fn main() {
  // Create progress state
  let progress_state = Arc::new(Mutex::new(0));
  
  tauri::Builder::default()
      .manage(ProgressState(progress_state))
      .invoke_handler(tauri::generate_handler![
          start_download,
          get_progress,
          is_pro,
          activate_license,
          license_info
      ])
      .run(tauri::generate_context!())
      .expect("error while running tauri application");
}