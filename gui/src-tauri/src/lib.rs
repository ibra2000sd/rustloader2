use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use tauri::{AppHandle, Manager, Window, Emitter};
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

// Import our notification module
pub mod notification;
use notification::{NotificationManager, NotificationOptions, NotificationType, NotificationState};

// Performance-optimized progress tracking for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub id: String,
    pub progress: f64,
    pub file_name: String,
    pub file_size: u64,
    pub downloaded_size: u64,
    pub speed: f64,
    pub time_remaining: Option<u64>,
    pub status: String,
}

// Use a throttled sender for performance optimization
pub struct ThrottledSender {
    app: AppHandle,
    last_update: Mutex<Instant>,
    min_interval: Duration,
    pending_updates: Mutex<Vec<DownloadProgress>>,
    batch_update_in_progress: AtomicBool,
}

impl ThrottledSender {
    pub fn new(app: AppHandle, min_interval_ms: u64) -> Self {
        Self {
            app,
            last_update: Mutex::new(Instant::now()),
            min_interval: Duration::from_millis(min_interval_ms),
            pending_updates: Mutex::new(Vec::new()),
            batch_update_in_progress: AtomicBool::new(false),
        }
    }

    pub fn send_progress(&self, progress: DownloadProgress) {
        // Add to pending updates
        {
            let mut updates = self.pending_updates.lock().unwrap();
            
            // Check if we already have an update for this ID
            let existing_index = updates.iter().position(|p| p.id == progress.id);
            if let Some(idx) = existing_index {
                // Replace the existing update
                updates[idx] = progress;
            } else {
                // Add a new update
                updates.push(progress);
            }
        }
        
        // Check if we need to trigger a batch update
        let should_send = {
            let mut last_update = self.last_update.lock().unwrap();
            let now = Instant::now();
            let elapsed = now.duration_since(*last_update);
            if elapsed >= self.min_interval {
                *last_update = now;
                true
            } else {
                false
            }
        };
        
        if should_send && !self.batch_update_in_progress.load(Ordering::Relaxed) {
            self.flush_updates();
        }
    }
    
    pub fn flush_updates(&self) {
        if self.batch_update_in_progress.compare_exchange(
            false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            return; // Another flush is already in progress
        }
        
        let updates = {
            let mut pending = self.pending_updates.lock().unwrap();
            if pending.is_empty() {
                self.batch_update_in_progress.store(false, Ordering::Release);
                return; // Nothing to send
            }
            
            // Take the updates and reset the pending list
            std::mem::take(&mut *pending)
        };
        
        // Clone the app handle to use in the async task
        let app_clone = self.app.clone();
        let sender_clone = self.clone();
        
        // Spawn a task to send the updates without blocking
        tauri::async_runtime::spawn(async move {
            if let Err(e) = app_clone.emit("download-progress-batch", updates) {
                eprintln!("Error sending progress updates: {}", e);
            }
            
            // Mark as complete
            sender_clone.batch_update_in_progress.store(false, Ordering::Release);
        });
    }
}

impl Clone for ThrottledSender {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            last_update: Mutex::new(*self.last_update.lock().unwrap()),
            min_interval: self.min_interval,
            pending_updates: Mutex::new(Vec::new()),
            batch_update_in_progress: AtomicBool::new(false),
        }
    }
}

// Create a global state to track downloads
#[derive(Clone)]
pub struct DownloadManagerState {
    pub progress_sender: Arc<ThrottledSender>,
    pub cancellation_channels: Arc<Mutex<std::collections::HashMap<String, mpsc::Sender<()>>>>,
}

impl DownloadManagerState {
    pub fn new(app: AppHandle) -> Self {
        Self {
            progress_sender: Arc::new(ThrottledSender::new(app, 100)), // Update UI at max 10fps
            cancellation_channels: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
    
    pub fn register_download(&self, id: &str, cancel_tx: mpsc::Sender<()>) {
        let mut channels = self.cancellation_channels.lock().unwrap();
        channels.insert(id.to_string(), cancel_tx);
    }
    
    pub fn unregister_download(&self, id: &str) {
        let mut channels = self.cancellation_channels.lock().unwrap();
        channels.remove(id);
    }
    
    pub fn cancel_download(&self, id: &str) -> Result<(), String> {
        let channels = self.cancellation_channels.lock().unwrap();
        if let Some(tx) = channels.get(id) {
            if let Err(e) = tx.try_send(()) {
                return Err(format!("Failed to send cancellation signal: {}", e));
            }
            Ok(())
        } else {
            Err(format!("Download with ID {} not found", id))
        }
    }
    
    pub fn update_progress(&self, progress: DownloadProgress) {
        self.progress_sender.send_progress(progress);
    }
    
    pub fn force_flush(&self) {
        self.progress_sender.flush_updates();
    }
    
    // The struct is now cloneable, so this method is not needed anymore
    // but we'll keep it for backward compatibility
    pub fn inner(&self) -> &Self {
        self
    }
}

// Helper to create an optimized download manager window
pub fn create_optimized_window(app: &AppHandle, label: &str) -> tauri::Result<Window> {
    // In Tauri 2.x, window building is handled differently
    // For simplicity, we'll just get an existing window or log an error
    match app.get_window(label) {
        Some(window) => {
            // In a real application, we would add initialization for the window here
            Ok(window)
        },
        None => {
            // In a real application, this would create a new window
            // using the appropriate Tauri 2.x API
            log::error!("Window {} does not exist", label);
            Err(tauri::Error::WindowNotFound)
        }
    }
}

// Utility functions for performance optimization
pub mod utils {
    use std::time::{Duration, Instant};
    use std::sync::Mutex;
    
    // Rate limiter for UI updates to prevent excessive rendering
    pub struct RateLimiter {
        last_update: Mutex<Instant>,
        min_interval: Duration,
    }
    
    impl RateLimiter {
        pub fn new(min_interval_ms: u64) -> Self {
            Self {
                last_update: Mutex::new(Instant::now()),
                min_interval: Duration::from_millis(min_interval_ms),
            }
        }
        
        pub fn should_update(&self) -> bool {
            let mut last_update = self.last_update.lock().unwrap();
            let now = Instant::now();
            let elapsed = now.duration_since(*last_update);
            
            if elapsed >= self.min_interval {
                *last_update = now;
                true
            } else {
                false
            }
        }
    }
    
    // Timer to measure and optimize performance bottlenecks
    pub struct PerfTimer {
        start: Instant,
        name: String,
    }
    
    impl PerfTimer {
        pub fn new(name: &str) -> Self {
            Self {
                start: Instant::now(),
                name: name.to_string(),
            }
        }
    }
    
    impl Drop for PerfTimer {
        fn drop(&mut self) {
            let elapsed = self.start.elapsed();
            // Only log if operation took over 50ms (potential performance issue)
            if elapsed > Duration::from_millis(50) {
                eprintln!("⚠️ Performance warning: {} took {:?}", self.name, elapsed);
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      
      // Register the download manager state
      app.manage(DownloadManagerState::new(app.handle().clone()));
      
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
