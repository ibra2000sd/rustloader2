use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use once_cell::sync::Lazy;

// Make modules accessible in tests
pub mod cli;
pub mod dependency_validator;
pub mod downloader;
pub mod download_manager;
pub mod error;
pub mod license;
pub mod security;
pub mod utils;
pub mod version;

// Re-export download manager types for easier use
pub use crate::download_manager::{
    DownloadItem, DownloadPriority, DownloadQueue, DownloadStatus, 
    add_download_to_queue, pause_all_downloads, resume_all_downloads,
    pause_download, resume_download, cancel_download, 
    set_download_priority, get_all_downloads, get_download_status,
    shutdown_download_manager,
};

// Progress tracking state using atomics for thread safety
static DOWNLOAD_PROGRESS: Lazy<ProgressState> = Lazy::new(|| {
    ProgressState {
        progress: AtomicU64::new(0),
        downloaded_bytes: AtomicU64::new(0),
        total_bytes: AtomicU64::new(0),
        speed: Mutex::new(0.0),
        eta: Mutex::new(None),
        filename: Mutex::new(String::new()),
        last_update: Mutex::new(Instant::now()),
    }
});

struct ProgressState {
    progress: AtomicU64,
    downloaded_bytes: AtomicU64,
    total_bytes: AtomicU64,
    speed: Mutex<f64>,
    eta: Mutex<Option<Duration>>,
    filename: Mutex<String>,
    last_update: Mutex<Instant>,
}

#[derive(Serialize, Deserialize)]
pub struct ProgressData {
    pub progress: u64,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    pub speed: f64,
    #[serde(rename = "timeRemaining")]
    pub time_remaining: Option<u64>,
}

// Function to update progress information (called from downloader module)
pub fn update_download_progress(
    progress: u64,
    _downloaded: u64,
    total: u64,
    current_speed: f64,
    filename: &str,
) {
    DOWNLOAD_PROGRESS.progress.store(progress, Ordering::SeqCst);
    DOWNLOAD_PROGRESS.downloaded_bytes.store(_downloaded, Ordering::SeqCst);
    DOWNLOAD_PROGRESS.total_bytes.store(total, Ordering::SeqCst);
    
    let mut speed = DOWNLOAD_PROGRESS.speed.lock().unwrap();
    *speed = current_speed;
    
    let mut eta = DOWNLOAD_PROGRESS.eta.lock().unwrap();
    if total > 0 && current_speed > 0.0 {
        let remaining_bytes = total.saturating_sub(_downloaded) as f64;
        let seconds_remaining = remaining_bytes / current_speed;
        *eta = Some(std::time::Duration::from_secs_f64(seconds_remaining));
    } else {
        *eta = None;
    }
    
    if !filename.is_empty() {
        let mut current_filename = DOWNLOAD_PROGRESS.filename.lock().unwrap();
        *current_filename = filename.to_string();
    }
    
    let mut last_update = DOWNLOAD_PROGRESS.last_update.lock().unwrap();
    *last_update = std::time::Instant::now();
}


// Function to get current progress (called from GUI)
pub fn get_download_progress() -> Result<ProgressData, String> {
    let progress = DOWNLOAD_PROGRESS.progress.load(Ordering::SeqCst);
    let _downloaded = DOWNLOAD_PROGRESS.downloaded_bytes.load(Ordering::SeqCst);
    let total = DOWNLOAD_PROGRESS.total_bytes.load(Ordering::SeqCst);
    
    let speed = *DOWNLOAD_PROGRESS.speed.lock().map_err(|e| e.to_string())?;
    let eta = DOWNLOAD_PROGRESS.eta.lock().map_err(|e| e.to_string())?.map(|d| d.as_secs());
    let filename = DOWNLOAD_PROGRESS.filename.lock().map_err(|e| e.to_string())?.clone();
    
    let last_update = *DOWNLOAD_PROGRESS.last_update.lock().map_err(|e| e.to_string())?;
    if last_update.elapsed() > Duration::from_secs(5) && progress > 0 && progress < 100 {
        return Err("Download progress information is stale".to_string());
    }
    
    Ok(ProgressData {
        progress,
        file_name: filename,
        file_size: total,
        speed,
        time_remaining: eta,
    })
}

// Function to reset progress tracking (called when starting a new download)
pub fn reset_download_progress() {
    DOWNLOAD_PROGRESS.progress.store(0, Ordering::SeqCst);
    DOWNLOAD_PROGRESS.downloaded_bytes.store(0, Ordering::SeqCst);
    DOWNLOAD_PROGRESS.total_bytes.store(0, Ordering::SeqCst);
    
    let mut speed = DOWNLOAD_PROGRESS.speed.lock().unwrap();
    *speed = 0.0;
    
    let mut eta = DOWNLOAD_PROGRESS.eta.lock().unwrap();
    *eta = None;
    
    let mut filename = DOWNLOAD_PROGRESS.filename.lock().unwrap();
    *filename = String::new();
    
    let mut last_update = DOWNLOAD_PROGRESS.last_update.lock().unwrap();
    *last_update = Instant::now();
}
