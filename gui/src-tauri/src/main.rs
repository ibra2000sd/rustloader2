#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{Runtime, State, Window};
use rustloader::{
  download_video,
  check_is_pro,
  activate_pro_license,
  get_license_info,
  get_download_progress
};

struct ProgressState(Arc<Mutex<i32>>);

#[tauri::command]
fn start_download<R: Runtime>(
  window: Window<R>,
  url: String,
  quality: Option<String>,
  format: String,
  start_time: Option<String>,
  end_time: Option<String>,
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

  let _ = window.emit("download-progress", serde_json::json!({
    "progress": 0,
    "fileName": "Initializing download...",
    "fileSize": 0,
    "speed": 0,
    "timeRemaining": 0
  }));

  thread::spawn(move || {
    match download_video(
      &url_copy,
      quality.as_deref(),
      &format,
      start_time,
      end_time,
      use_playlist,
      download_subtitles,
      output_dir,
    ) {
      Ok(result) => {
        let _ = window_copy.emit("download-progress", serde_json::json!({
          "progress": 100,
          "fileName": "Download complete",
          "fileSize": 0,
          "speed": 0,
          "timeRemaining": 0
        }));
        let _ = window_copy.emit("download-completed", serde_json::json!({
          "success": true,
          "message": result
        }));
      },
      Err(error) => {
        let _ = window_copy.emit("download-completed", serde_json::json!({
          "success": false,
          "message": error.to_string()
        }));
      }
    }
  });

  Ok(())
}

#[tauri::command]
fn poll_download_progress<R: Runtime>(window: Window<R>) {
  thread::spawn(move || {
    loop {
      match get_download_progress() {
        Ok(progress_data) => {
          let _ = window.emit("download-progress", progress_data);
          if progress_data.progress >= 100 {
            break;
          }
        },
        Err(_) => {
          thread::sleep(std::time::Duration::from_millis(500));
        }
      }
      thread::sleep(std::time::Duration::from_millis(500));
    }
  });
}

#[tauri::command]
fn get_video_info(url: String) -> Result<VideoInfo, String> {
    let output = std::process::Command::new("yt-dlp")
        .args(["--dump-json", "--no-playlist", &url])
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
  false
}

fn main() {
  let progress_state = Arc::new(Mutex::new(0));

  tauri::Builder::default()
      .manage(ProgressState(progress_state))
      .invoke_handler(tauri::generate_handler![
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
