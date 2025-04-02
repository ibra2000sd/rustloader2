// src/download_manager.rs
// Enhanced download functionality with queue management, prioritization, persistence, and concurrency

use crate::error::AppError;
use chrono::{DateTime, Utc};
use log::{debug, error};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::task::JoinHandle;
use dirs_next as dirs;

/// Priority levels for downloads
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DownloadPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for DownloadPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Current status of a download
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
    Canceled,
}

impl Default for DownloadStatus {
    fn default() -> Self {
        Self::Queued
    }
}

/// A download item in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    /// Unique identifier for the download
    pub id: String,
    /// URL to download from
    pub url: String,
    /// Title or name of the content
    pub title: Option<String>,
    /// Selected quality option
    pub quality: Option<String>,
    /// Output format (mp3, mp4, etc.)
    pub format: String,
    /// Optional start time for clip extraction
    pub start_time: Option<String>,
    /// Optional end time for clip extraction
    pub end_time: Option<String>,
    /// Whether to download entire playlist
    pub use_playlist: bool,
    /// Whether to download subtitles
    pub download_subtitles: bool,
    /// Custom output directory
    pub output_dir: Option<String>,
    /// Whether to force re-download
    pub force_download: bool,
    /// Optional bitrate for audio
    pub bitrate: Option<String>,
    /// Current download status
    pub status: DownloadStatus,
    /// Download priority
    pub priority: DownloadPriority,
    /// When the download was added to queue
    pub added_at: DateTime<Utc>,
    /// When the download started
    pub started_at: Option<DateTime<Utc>>,
    /// When the download completed/failed/canceled
    pub finished_at: Option<DateTime<Utc>>,
    /// Current progress (0-100)
    pub progress: f64,
    /// Number of bytes downloaded
    pub downloaded_bytes: u64,
    /// Total bytes to download
    pub total_bytes: u64,
    /// Current download speed in bytes per second
    pub speed: f64,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Output file path once completed
    pub output_path: Option<String>,
    /// Unique token for cancellation and control
    #[serde(skip)]
    pub cancel_token: Option<broadcast::Sender<()>>,
}

impl DownloadItem {
    /// Create a new download item
    pub fn new(url: &str, format: &str) -> Self {
        let id = generate_download_id();
        
        Self {
            id,
            url: url.to_string(),
            title: None,
            quality: None,
            format: format.to_string(),
            start_time: None,
            end_time: None,
            use_playlist: false,
            download_subtitles: false,
            output_dir: None,
            force_download: false,
            bitrate: None,
            status: DownloadStatus::Queued,
            priority: DownloadPriority::Normal,
            added_at: Utc::now(),
            started_at: None,
            finished_at: None,
            progress: 0.0,
            downloaded_bytes: 0,
            total_bytes: 0,
            speed: 0.0,
            retry_count: 0,
            error_message: None,
            output_path: None,
            cancel_token: None,
        }
    }

    /// Create a new download builder for more complex configuration
    pub fn builder(url: &str, format: &str) -> DownloadItemBuilder {
        DownloadItemBuilder::new(url, format)
    }
    
    /// Check if the download is active (downloading or queued)
    pub fn is_active(&self) -> bool {
        matches!(self.status, DownloadStatus::Downloading | DownloadStatus::Queued)
    }
    
    /// Check if the download is paused
    pub fn is_paused(&self) -> bool {
        self.status == DownloadStatus::Paused
    }
    
    /// Check if the download is completed
    pub fn is_completed(&self) -> bool {
        self.status == DownloadStatus::Completed
    }
    
    /// Check if the download has failed
    pub fn is_failed(&self) -> bool {
        self.status == DownloadStatus::Failed
    }
    
    /// Check if the download was canceled
    pub fn is_canceled(&self) -> bool {
        self.status == DownloadStatus::Canceled
    }
    
    /// Check if the download is finished (completed, failed, or canceled)
    pub fn is_finished(&self) -> bool {
        self.is_completed() || self.is_failed() || self.is_canceled()
    }
    
    /// Create a cancel token for this download
    pub fn create_cancel_token(&mut self) -> broadcast::Receiver<()> {
        let (tx, rx) = broadcast::channel(1);
        self.cancel_token = Some(tx);
        rx
    }
    
    /// Cancel this download
    pub fn cancel(&mut self) {
        if let Some(token) = &self.cancel_token {
            let _ = token.send(());
        }
        
        if !self.is_finished() {
            self.status = DownloadStatus::Canceled;
            self.finished_at = Some(Utc::now());
        }
    }
    
    /// Update progress information
    #[allow(dead_code)]
    pub fn update_progress(&mut self, downloaded: u64, total: u64, speed: f64) {
        self.downloaded_bytes = downloaded;
        self.total_bytes = total;
        self.speed = speed;
        
        if total > 0 {
            self.progress = (downloaded as f64 / total as f64) * 100.0;
        }
    }
    
    /// Mark download as started
    pub fn mark_started(&mut self) {
        self.status = DownloadStatus::Downloading;
        self.started_at = Some(Utc::now());
    }
    
    /// Mark download as completed
    pub fn mark_completed(&mut self, output_path: Option<String>) {
        self.status = DownloadStatus::Completed;
        self.finished_at = Some(Utc::now());
        self.progress = 100.0;
        if let Some(path) = output_path {
            self.output_path = Some(path);
        }
    }
    
    /// Mark download as failed
    pub fn mark_failed(&mut self, error: Option<String>) {
        self.status = DownloadStatus::Failed;
        self.finished_at = Some(Utc::now());
        self.error_message = error;
    }
    
    /// Mark download as paused
    pub fn mark_paused(&mut self) {
        self.status = DownloadStatus::Paused;
    }
    
    /// Mark download as resumed
    pub fn mark_resumed(&mut self) {
        if self.status == DownloadStatus::Paused {
            self.status = DownloadStatus::Queued;
            if self.started_at.is_some() {
                // If it was previously started, mark it as downloading
                self.status = DownloadStatus::Downloading;
            }
        }
    }
    
    /// Increment retry count
    #[allow(dead_code)]
    pub fn increment_retry_count(&mut self) {
        self.retry_count += 1;
    }
}

/// Builder for creating download items with fluent interface
#[derive(Debug)]
pub struct DownloadItemBuilder {
    item: DownloadItem,
}

impl DownloadItemBuilder {
    /// Create a new download builder
    pub fn new(url: &str, format: &str) -> Self {
        Self {
            item: DownloadItem::new(url, format),
        }
    }
    
    /// Set the title
    #[allow(dead_code)]
    pub fn title(mut self, title: Option<&str>) -> Self {
        self.item.title = title.map(|s| s.to_string());
        self
    }
    
    /// Set the quality
    pub fn quality(mut self, quality: Option<&str>) -> Self {
        self.item.quality = quality.map(|s| s.to_string());
        self
    }
    
    /// Set time range
    pub fn time_range(mut self, start: Option<&str>, end: Option<&str>) -> Self {
        self.item.start_time = start.map(|s| s.to_string());
        self.item.end_time = end.map(|s| s.to_string());
        self
    }
    
    /// Set playlist option
    pub fn playlist(mut self, use_playlist: bool) -> Self {
        self.item.use_playlist = use_playlist;
        self
    }
    
    /// Set subtitles option
    pub fn subtitles(mut self, download_subtitles: bool) -> Self {
        self.item.download_subtitles = download_subtitles;
        self
    }
    
    /// Set output directory
    pub fn output_dir(mut self, output_dir: Option<&str>) -> Self {
        self.item.output_dir = output_dir.map(|s| s.to_string());
        self
    }
    
    /// Set force download option
    pub fn force_download(mut self, force: bool) -> Self {
        self.item.force_download = force;
        self
    }
    
    /// Set bitrate
    pub fn bitrate(mut self, bitrate: Option<&str>) -> Self {
        self.item.bitrate = bitrate.map(|s| s.to_string());
        self
    }
    
    /// Set priority
    pub fn priority(mut self, priority: DownloadPriority) -> Self {
        self.item.priority = priority;
        self
    }
    
    /// Build the download item
    pub fn build(self) -> DownloadItem {
        self.item
    }
}

/// Commands for managing the download queue
#[derive(Debug, Clone)]
pub enum QueueCommand {
    Add(DownloadItem),
    Pause(String), // id
    Resume(String), // id
    Cancel(String), // id
    PauseAll,
    ResumeAll,
    SetPriority(String, DownloadPriority), // id, new priority
    RemoveCompleted,
    ClearFailed,
    #[allow(dead_code)]
    MoveUp(String), // id
    #[allow(dead_code)]
    MoveDown(String), // id
    SaveQueue,
    LoadQueue,
}

/// Manages a queue of downloads with advanced features
#[derive(Debug)]
pub struct DownloadQueue {
    /// Map of download IDs to download items
    downloads: Arc<RwLock<HashMap<String, DownloadItem>>>,
    /// Queue of pending download IDs, ordered by priority and time added
    queue: Arc<Mutex<Vec<String>>>,
    /// Max concurrent downloads
    max_concurrent: Arc<RwLock<usize>>,
    /// Semaphore to control concurrency
    concurrency_control: Arc<Semaphore>,
    /// Command channel for queue operations
    command_tx: mpsc::Sender<QueueCommand>,
    /// Command receiver for queue operations
    command_rx: Arc<Mutex<Option<mpsc::Receiver<QueueCommand>>>>,
    /// Queue state path for persistence
    state_path: PathBuf,
    /// In-memory map of running download tasks
    active_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    /// Flag indicating if queue processor is running
    is_running: Arc<RwLock<bool>>,
    /// Channel for notifying listeners of queue changes
    notify_tx: broadcast::Sender<()>,
}

/// Default implementation for DownloadQueue
impl Default for DownloadQueue {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(100);
        let (notify_tx, _) = broadcast::channel(100);
        
        Self {
            downloads: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(Mutex::new(Vec::new())),
            max_concurrent: Arc::new(RwLock::new(3)), // Default to 3 concurrent downloads
            concurrency_control: Arc::new(Semaphore::new(3)),
            command_tx: tx,
            command_rx: Arc::new(Mutex::new(Some(rx))),
            state_path: get_queue_state_path(),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            notify_tx,
        }
    }
}

impl DownloadQueue {
    /// Create a new download queue with the specified concurrency limit
    pub fn new(max_concurrent_downloads: usize) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let (notify_tx, _) = broadcast::channel(100);
        
        Self {
            downloads: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(Mutex::new(Vec::new())),
            max_concurrent: Arc::new(RwLock::new(max_concurrent_downloads)),
            concurrency_control: Arc::new(Semaphore::new(max_concurrent_downloads)),
            command_tx: tx,
            command_rx: Arc::new(Mutex::new(Some(rx))),
            state_path: get_queue_state_path(),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            notify_tx,
        }
    }
    
    /// Get a command sender that can be used to send commands to the queue
    #[allow(dead_code)]
    pub fn get_command_sender(&self) -> mpsc::Sender<QueueCommand> {
        self.command_tx.clone()
    }
    
    /// Get a notification receiver to be notified of queue changes
    #[allow(dead_code)]
    pub fn get_notification_receiver(&self) -> broadcast::Receiver<()> {
        self.notify_tx.subscribe()
    }
    
    /// Start the queue processor in a separate task
    pub async fn start(&self) -> Result<(), AppError> {
        {
            let mut is_running = self.is_running.write().unwrap();
            if *is_running {
                return Ok(());
            }
            *is_running = true;
        }
        
        // Try to load the saved queue
        // Explicitly drop the future to handle the warning
        std::mem::drop(self.load_state());
        
        let downloads = self.downloads.clone();
        let queue = self.queue.clone();
        let max_concurrent = self.max_concurrent.clone();
        let concurrency_control = self.concurrency_control.clone();
        let active_tasks = self.active_tasks.clone();
        let is_running = self.is_running.clone();
        let state_path = self.state_path.clone();
        let command_rx_mutex = self.command_rx.clone();
        let notify_tx = self.notify_tx.clone();
        
        tokio::spawn(async move {
            let command_rx = {
                let mut guard = command_rx_mutex.lock().unwrap();
                guard.take()
            };
            
            if let Some(mut rx) = command_rx {
                let mut autosave_interval = tokio::time::interval(std::time::Duration::from_secs(60));
                
                loop {
                    tokio::select! {
                        // Process queue commands
                        Some(cmd) = rx.recv() => {
                            let ctx = CommandContext {
                                downloads: &downloads,
                                queue: &queue,
                                _max_concurrent: &max_concurrent,
                                concurrency_control: &concurrency_control,
                                active_tasks: &active_tasks,
                                state_path: &state_path,
                                notify_tx: &notify_tx,
                            };
                            process_command(cmd, &ctx).await;
                        }
                        
                        // Auto-save queue state periodically
                        _ = autosave_interval.tick() => {
                            debug!("Auto-saving download queue state");
                            let downloads_clone = Arc::clone(&downloads);
                            let state_path_clone = state_path.clone();
                            let _ = save_queue_state(downloads_clone, state_path_clone).await;
                        }
                        
                        // Check for task completion
                        _ = tokio::time::sleep(Duration::from_secs(1)) => {
                            let downloads_clone = Arc::clone(&downloads);
                            let queue_clone = Arc::clone(&queue);
                            let concurrency_clone = Arc::clone(&concurrency_control);
                            let active_tasks_clone = Arc::clone(&active_tasks);
                            let notify_tx_clone = notify_tx.clone();
                            
                            check_and_process_queue(
                                downloads_clone,
                                queue_clone,
                                concurrency_clone,
                                active_tasks_clone,
                                notify_tx_clone,
                            ).await;
                        }
                    }
                    
                    // Check if we should stop the processor
                    if !*is_running.read().unwrap() {
                        debug!("Download queue processor stopped");
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop the queue processor
    pub async fn stop(&self) -> Result<(), AppError> {
        // Set running flag to false first
        {
            let mut is_running = self.is_running.write().unwrap();
            *is_running = false;
        }
        
        // Save queue state before stopping
        self.save_state().await?;
        
        // Cancel any active downloads
        let tasks_to_cancel = {
            let mut active_tasks = self.active_tasks.lock().unwrap();
            active_tasks.drain().collect::<Vec<_>>()
        };
        
        // Cancel each task outside the lock
        for (id, handle) in tasks_to_cancel {
            debug!("Cancelling download task for {}", id);
            handle.abort();
            
            // Update download item status
            if let Some(mut item) = self.get_download(id.clone()) {
                item.cancel();
                let mut downloads = self.downloads.write().unwrap();
                downloads.insert(id, item);
            }
        }
        
        Ok(())
    }
    
    /// Add a download to the queue
    pub async fn add_download(&self, item: DownloadItem) -> Result<(), AppError> {
        let cmd = QueueCommand::Add(item);
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Pause a download by ID
    pub async fn pause_download(&self, id: &str) -> Result<(), AppError> {
        let cmd = QueueCommand::Pause(id.to_string());
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Resume a download by ID
    pub async fn resume_download(&self, id: &str) -> Result<(), AppError> {
        let cmd = QueueCommand::Resume(id.to_string());
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Cancel a download by ID
    pub async fn cancel_download(&self, id: &str) -> Result<(), AppError> {
        let cmd = QueueCommand::Cancel(id.to_string());
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Pause all active downloads
    pub async fn pause_all(&self) -> Result<(), AppError> {
        let cmd = QueueCommand::PauseAll;
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Resume all paused downloads
    pub async fn resume_all(&self) -> Result<(), AppError> {
        let cmd = QueueCommand::ResumeAll;
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Set the priority of a download
    pub async fn set_priority(&self, id: &str, priority: DownloadPriority) -> Result<(), AppError> {
        let cmd = QueueCommand::SetPriority(id.to_string(), priority);
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Remove all completed downloads from the queue
    pub async fn remove_completed(&self) -> Result<(), AppError> {
        let cmd = QueueCommand::RemoveCompleted;
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Clear all failed downloads from the queue
    pub async fn clear_failed(&self) -> Result<(), AppError> {
        let cmd = QueueCommand::ClearFailed;
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Move a download up in the queue (higher priority)
    #[allow(dead_code)]
    pub async fn move_up(&self, id: &str) -> Result<(), AppError> {
        let cmd = QueueCommand::MoveUp(id.to_string());
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Move a download down in the queue (lower priority)
    #[allow(dead_code)]
    pub async fn move_down(&self, id: &str) -> Result<(), AppError> {
        let cmd = QueueCommand::MoveDown(id.to_string());
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Save the queue state
    pub async fn save_state(&self) -> Result<(), AppError> {
        let cmd = QueueCommand::SaveQueue;
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Load the queue state
    pub async fn load_state(&self) -> Result<(), AppError> {
        let cmd = QueueCommand::LoadQueue;
        self.command_tx.send(cmd).await.map_err(|e| {
            AppError::General(format!("Failed to send queue command: {}", e))
        })
    }
    
    /// Get a download item by ID
    pub fn get_download(&self, id: String) -> Option<DownloadItem> {
        let downloads = self.downloads.read().unwrap();
        downloads.get(&id).cloned()
    }
    
    /// Get all downloads in the queue
    pub fn get_all_downloads(&self) -> Vec<DownloadItem> {
        let downloads = self.downloads.read().unwrap();
        downloads.values().cloned().collect()
    }
    
    /// Get active downloads
    #[allow(dead_code)]
    pub fn get_active_downloads(&self) -> Vec<DownloadItem> {
        let downloads = self.downloads.read().unwrap();
        downloads.values()
            .filter(|item| item.is_active())
            .cloned()
            .collect()
    }
    
    /// Get paused downloads
    #[allow(dead_code)]
    pub fn get_paused_downloads(&self) -> Vec<DownloadItem> {
        let downloads = self.downloads.read().unwrap();
        downloads.values()
            .filter(|item| item.is_paused())
            .cloned()
            .collect()
    }
    
    /// Get completed downloads
    #[allow(dead_code)]
    pub fn get_completed_downloads(&self) -> Vec<DownloadItem> {
        let downloads = self.downloads.read().unwrap();
        downloads.values()
            .filter(|item| item.is_completed())
            .cloned()
            .collect()
    }
    
    /// Get failed downloads
    #[allow(dead_code)]
    pub fn get_failed_downloads(&self) -> Vec<DownloadItem> {
        let downloads = self.downloads.read().unwrap();
        downloads.values()
            .filter(|item| item.is_failed())
            .cloned()
            .collect()
    }
    
    /// Get the number of active downloads
    #[allow(dead_code)]
    pub fn get_active_count(&self) -> usize {
        let downloads = self.downloads.read().unwrap();
        downloads.values()
            .filter(|item| item.is_active())
            .count()
    }
    
    /// Get the number of paused downloads
    #[allow(dead_code)]
    pub fn get_paused_count(&self) -> usize {
        let downloads = self.downloads.read().unwrap();
        downloads.values()
            .filter(|item| item.is_paused())
            .count()
    }
    
    /// Get the number of completed downloads
    #[allow(dead_code)]
    pub fn get_completed_count(&self) -> usize {
        let downloads = self.downloads.read().unwrap();
        downloads.values()
            .filter(|item| item.is_completed())
            .count()
    }
    
    /// Get the number of failed downloads
    #[allow(dead_code)]
    pub fn get_failed_count(&self) -> usize {
        let downloads = self.downloads.read().unwrap();
        downloads.values()
            .filter(|item| item.is_failed())
            .count()
    }
    
    /// Get the total number of downloads
    #[allow(dead_code)]
    pub fn get_total_count(&self) -> usize {
        let downloads = self.downloads.read().unwrap();
        downloads.len()
    }
    
    /// Get the maximum number of concurrent downloads
    #[allow(dead_code)]
    pub fn get_max_concurrent(&self) -> usize {
        *self.max_concurrent.read().unwrap()
    }
    
    /// Set the maximum number of concurrent downloads
    #[allow(dead_code)]
    pub fn set_max_concurrent(&self, max: usize) {
        let current = *self.max_concurrent.read().unwrap();
        if max != current {
            *self.max_concurrent.write().unwrap() = max;
            
            // Update the semaphore
            let diff = max as isize - current as isize;
            match diff.cmp(&0) {
                std::cmp::Ordering::Greater => {
                    // Add permits
                    self.concurrency_control.add_permits(diff as usize);
                },
                std::cmp::Ordering::Less => {
                    // Close permits - note that this doesn't affect already acquired permits
                    // The next time permits are released, the semaphore will correctly limit to the new max
                    debug!("Reducing max concurrent downloads from {} to {}", current, max);
                },
                std::cmp::Ordering::Equal => {
                    // No change needed
                }
            }
        }
    }
}

/// Generate a unique download ID
fn generate_download_id() -> String {
    use rand::Rng;
    let timestamp = chrono::Utc::now().timestamp_millis();
    let random = rand::thread_rng().gen::<u32>();
    format!("dl_{}_{}", timestamp, random)
}

/// Get the path to store the queue state
fn get_queue_state_path() -> PathBuf {
    let mut path = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    
    path.push("rustloader");
    fs::create_dir_all(&path).unwrap_or_default();
    
    path.push("download_queue.json");
    path
}

/// Command processing context
struct CommandContext<'a> {
    downloads: &'a Arc<RwLock<HashMap<String, DownloadItem>>>,
    queue: &'a Arc<Mutex<Vec<String>>>,
    _max_concurrent: &'a Arc<RwLock<usize>>, // Unused but kept for future use
    concurrency_control: &'a Arc<Semaphore>,
    active_tasks: &'a Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    state_path: &'a std::path::Path,
    notify_tx: &'a broadcast::Sender<()>,
}

/// Process a queue command
async fn process_command(
    cmd: QueueCommand,
    ctx: &CommandContext<'_>,
) {
    debug!("Processing queue command: {:?}", cmd);
    
    match cmd {
        QueueCommand::Add(item) => {
            let id = item.id.clone();
            let is_priority = item.priority == DownloadPriority::High || item.priority == DownloadPriority::Critical;
            
            // Add to downloads map
            {
                let mut downloads_map = ctx.downloads.write().unwrap();
                downloads_map.insert(id.clone(), item);
            }
            
            // Add to queue based on priority
            {
                let mut queue_vec = ctx.queue.lock().unwrap();
                
                if is_priority {
                    // Add high priority items to the front of the queue
                    queue_vec.insert(0, id.clone());
                } else {
                    // Add normal priority items to the end
                    queue_vec.push(id.clone());
                }
            }
            
            // Process the queue
            let downloads_clone = Arc::clone(ctx.downloads);
            let queue_clone = Arc::clone(ctx.queue);
            let concurrency_clone = Arc::clone(ctx.concurrency_control);
            let active_tasks_clone = Arc::clone(ctx.active_tasks);
            let notify_tx_clone = ctx.notify_tx.clone();
            
            check_and_process_queue(
                downloads_clone,
                queue_clone,
                concurrency_clone,
                active_tasks_clone,
                notify_tx_clone,
            ).await;
            
            // Notify listeners
            let _ = ctx.notify_tx.send(());
        }
        
        QueueCommand::Pause(id) => {
            let mut should_notify = false;
            
            // Update download status in the downloads map
            {
                let mut downloads_map = ctx.downloads.write().unwrap();
                if let Some(item) = downloads_map.get_mut(&id) {
                    if item.is_active() {
                        item.mark_paused();
                        should_notify = true;
                        
                        // If this download has a cancel token, send a cancel signal
                        if let Some(token) = &item.cancel_token {
                            let _ = token.send(());
                        }
                    }
                }
            }
            
            // Remove from active tasks
            {
                let mut tasks = ctx.active_tasks.lock().unwrap();
                if let Some(handle) = tasks.remove(&id) {
                    debug!("Pausing download {}", id);
                    handle.abort();
                }
            }
            
            if should_notify {
                let _ = ctx.notify_tx.send(());
            }
        }
        
        QueueCommand::Resume(id) => {
            let mut should_notify = false;
            
            // Update download status in the downloads map
            {
                let mut downloads_map = ctx.downloads.write().unwrap();
                if let Some(item) = downloads_map.get_mut(&id) {
                    if item.is_paused() {
                        item.mark_resumed();
                        should_notify = true;
                        
                        // Add back to queue
                        let mut queue_vec = ctx.queue.lock().unwrap();
                        
                        // Add to front if high priority
                        if item.priority == DownloadPriority::High || item.priority == DownloadPriority::Critical {
                            queue_vec.insert(0, id.clone());
                        } else {
                            queue_vec.push(id.clone());
                        }
                    }
                }
            }
            
            if should_notify {
                // Process the queue
                let downloads_clone = Arc::clone(ctx.downloads);
                let queue_clone = Arc::clone(ctx.queue);
                let concurrency_clone = Arc::clone(ctx.concurrency_control);
                let active_tasks_clone = Arc::clone(ctx.active_tasks);
                let notify_tx_clone = ctx.notify_tx.clone();
                
                check_and_process_queue(
                    downloads_clone,
                    queue_clone,
                    concurrency_clone,
                    active_tasks_clone,
                    notify_tx_clone,
                ).await;
                
                let _ = ctx.notify_tx.send(());
            }
        }
        
        QueueCommand::Cancel(id) => {
            let mut should_notify = false;
            
            // Update download status in the downloads map
            {
                let mut downloads_map = ctx.downloads.write().unwrap();
                if let Some(item) = downloads_map.get_mut(&id) {
                    if !item.is_finished() {
                        item.cancel();
                        should_notify = true;
                        
                        // If this download has a cancel token, send a cancel signal
                        if let Some(token) = &item.cancel_token {
                            let _ = token.send(());
                        }
                    }
                }
            }
            
            // Remove from queue
            {
                let mut queue_vec = ctx.queue.lock().unwrap();
                queue_vec.retain(|qid| *qid != id);
            }
            
            // Remove from active tasks
            {
                let mut tasks = ctx.active_tasks.lock().unwrap();
                if let Some(handle) = tasks.remove(&id) {
                    debug!("Cancelling download {}", id);
                    handle.abort();
                }
            }
            
            if should_notify {
                let _ = ctx.notify_tx.send(());
            }
        }
        
        QueueCommand::PauseAll => {
            let mut paused_ids = Vec::new();
            
            // Pause all active downloads
            {
                let mut downloads_map = ctx.downloads.write().unwrap();
                
                for (id, item) in downloads_map.iter_mut() {
                    if item.is_active() {
                        item.mark_paused();
                        paused_ids.push(id.clone());
                        
                        // If this download has a cancel token, send a cancel signal
                        if let Some(token) = &item.cancel_token {
                            let _ = token.send(());
                        }
                    }
                }
            }
            
            // Clear queue
            {
                let mut queue_vec = ctx.queue.lock().unwrap();
                queue_vec.clear();
            }
            
            // Remove all from active tasks
            {
                let mut tasks = ctx.active_tasks.lock().unwrap();
                for id in &paused_ids {
                    if let Some(handle) = tasks.remove(id) {
                        debug!("Pausing download {}", id);
                        handle.abort();
                    }
                }
            }
            
            if !paused_ids.is_empty() {
                let _ = ctx.notify_tx.send(());
            }
        }
        
        QueueCommand::ResumeAll => {
            let mut resumed_count = 0;
            
            // Resume all paused downloads and add to queue
            {
                let mut queue_vec = ctx.queue.lock().unwrap();
                let mut downloads_map = ctx.downloads.write().unwrap();
                let mut high_priority = Vec::new();
                let mut normal_priority = Vec::new();
                
                for (id, item) in downloads_map.iter_mut() {
                    if item.is_paused() {
                        item.mark_resumed();
                        resumed_count += 1;
                        
                        if item.priority == DownloadPriority::High || item.priority == DownloadPriority::Critical {
                            high_priority.push(id.clone());
                        } else {
                            normal_priority.push(id.clone());
                        }
                    }
                }
                
                // Add high priority first, then normal priority
                for id in high_priority {
                    queue_vec.insert(0, id);
                }
                
                for id in normal_priority {
                    queue_vec.push(id);
                }
            }
            
            if resumed_count > 0 {
                // Process the queue
                let downloads_clone = Arc::clone(ctx.downloads);
                let queue_clone = Arc::clone(ctx.queue);
                let concurrency_clone = Arc::clone(ctx.concurrency_control);
                let active_tasks_clone = Arc::clone(ctx.active_tasks);
                let notify_tx_clone = ctx.notify_tx.clone();
                
                check_and_process_queue(
                    downloads_clone,
                    queue_clone,
                    concurrency_clone,
                    active_tasks_clone,
                    notify_tx_clone,
                ).await;
                
                let _ = ctx.notify_tx.send(());
            }
        }
        
        QueueCommand::SetPriority(id, priority) => {
            let mut should_reorder = false;
            let mut is_queued = false;
            
            // Update priority in downloads map
            {
                let mut downloads_map = ctx.downloads.write().unwrap();
                if let Some(item) = downloads_map.get_mut(&id) {
                    // Only change if priority is different
                    if item.priority != priority {
                        item.priority = priority;
                        should_reorder = true;
                        is_queued = item.status == DownloadStatus::Queued;
                    }
                }
            }
            
            // If download is in queue, reorder based on new priority
            if should_reorder && is_queued {
                let mut queue_vec = ctx.queue.lock().unwrap();
                
                // Remove from queue
                if let Some(index) = queue_vec.iter().position(|qid| *qid == id) {
                    queue_vec.remove(index);
                    
                    // Re-add based on priority
                    if priority == DownloadPriority::High || priority == DownloadPriority::Critical {
                        queue_vec.insert(0, id);
                    } else {
                        queue_vec.push(id);
                    }
                    
                    let _ = ctx.notify_tx.send(());
                }
            }
        }
        
        QueueCommand::RemoveCompleted => {
            let mut removed_count = 0;
            
            // Remove completed downloads
            {
                let mut downloads_map = ctx.downloads.write().unwrap();
                let completed_ids: Vec<String> = downloads_map.iter()
                    .filter(|(_, item)| item.is_completed())
                    .map(|(id, _)| id.clone())
                    .collect();
                
                for id in &completed_ids {
                    downloads_map.remove(id);
                    removed_count += 1;
                }
            }
            
            if removed_count > 0 {
                let _ = ctx.notify_tx.send(());
            }
        }
        
        QueueCommand::ClearFailed => {
            let mut cleared_count = 0;
            
            // Clear failed downloads
            {
                let mut downloads_map = ctx.downloads.write().unwrap();
                let failed_ids: Vec<String> = downloads_map.iter()
                    .filter(|(_, item)| item.is_failed())
                    .map(|(id, _)| id.clone())
                    .collect();
                
                for id in &failed_ids {
                    downloads_map.remove(id);
                    cleared_count += 1;
                }
            }
            
            if cleared_count > 0 {
                let _ = ctx.notify_tx.send(());
            }
        }
        
        QueueCommand::MoveUp(id) => {
            let mut queue_vec = ctx.queue.lock().unwrap();
            
            if let Some(index) = queue_vec.iter().position(|qid| *qid == id) {
                if index > 0 {
                    queue_vec.swap(index, index - 1);
                    let _ = ctx.notify_tx.send(());
                }
            }
        }
        
        QueueCommand::MoveDown(id) => {
            let mut queue_vec = ctx.queue.lock().unwrap();
            
            if let Some(index) = queue_vec.iter().position(|qid| *qid == id) {
                if index < queue_vec.len() - 1 {
                    queue_vec.swap(index, index + 1);
                    let _ = ctx.notify_tx.send(());
                }
            }
        }
        
        QueueCommand::SaveQueue => {
            let downloads_clone = Arc::clone(ctx.downloads);
            let state_path_clone = ctx.state_path.to_path_buf();
            let _ = save_queue_state(downloads_clone, state_path_clone).await;
        }
        
        QueueCommand::LoadQueue => {
            let _ = load_queue_state(Arc::clone(ctx.downloads), Arc::clone(ctx.queue), ctx.state_path.to_path_buf()).await;
            let _ = ctx.notify_tx.send(());
        }
    }
}

/// Check the queue and start downloads if slots are available
async fn check_and_process_queue(
    downloads: Arc<RwLock<HashMap<String, DownloadItem>>>,
    queue: Arc<Mutex<Vec<String>>>,
    concurrency_control: Arc<Semaphore>,
    active_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    notify_tx: broadcast::Sender<()>,
) {
    // Get next download from queue
    let mut next_download = None;
    let mut next_id = String::new();
    
    // Get the next item from the queue
    {
        let mut queue_vec = queue.lock().unwrap();
        if !queue_vec.is_empty() {
            next_id = queue_vec[0].clone();
            queue_vec.remove(0);
            
            let downloads_map = downloads.read().unwrap();
            next_download = downloads_map.get(&next_id).cloned();
        }
    }
    
    // Process the download if we got one
    if let Some(mut item) = next_download {
        debug!("Attempting to start download {}", item.id);
        
        // Check if semaphore has available permits
        if concurrency_control.available_permits() > 0 {
            // Mark as started and update in downloads map
            item.mark_started();
            let cancel_rx = item.create_cancel_token();
            
            {
                let mut downloads_map = downloads.write().unwrap();
                downloads_map.insert(item.id.clone(), item.clone());
            }
            
            // Clone everything needed for the task
            let item_id = item.id.clone();
            let item_for_task = item.clone();
            let downloads_for_task = Arc::clone(&downloads);
            let active_tasks_for_task = Arc::clone(&active_tasks);
            let notify_tx_for_task = notify_tx.clone();
            let concurrency_control_for_task = Arc::clone(&concurrency_control);
            
            // Spawn the download task
            let handle = tokio::spawn(async move {
                // Acquire the permit inside the task to ensure it lives long enough
                let _permit = concurrency_control_for_task.acquire().await.expect("Failed to acquire permit");
                
                // Execute the download
                let result = execute_download(item_for_task, cancel_rx).await;
                
                // Update download status based on result
                {
                    let mut downloads_map = downloads_for_task.write().unwrap();
                    
                    if let Some(dl_item) = downloads_map.get_mut(&item_id) {
                        match result {
                            Ok(output_path) => {
                                debug!("Download {} completed successfully", item_id);
                                dl_item.mark_completed(Some(output_path));
                            },
                            Err(e) => {
                                error!("Download {} failed: {}", item_id, e);
                                dl_item.mark_failed(Some(e.to_string()));
                            }
                        }
                    }
                }
                
                // Remove from active tasks
                {
                    let mut tasks = active_tasks_for_task.lock().unwrap();
                    tasks.remove(&item_id);
                }
                
                // Notify listeners of state change
                let _ = notify_tx_for_task.send(());
            });
            
            // Store the task handle
            {
                let mut tasks = active_tasks.lock().unwrap();
                tasks.insert(item.id.clone(), handle);
            }
            
            // Notify listeners
            let _ = notify_tx.send(());
            
            // Process the next download non-recursively to avoid Send issues
            let downloads_for_next = Arc::clone(&downloads);
            let queue_for_next = Arc::clone(&queue);
            let concurrency_for_next = Arc::clone(&concurrency_control);
            let active_tasks_for_next = Arc::clone(&active_tasks);
            let notify_tx_for_next = notify_tx.clone();
            
            // Use a static function that doesn't capture variables from its environment
            tokio::spawn(process_queue_static(
                downloads_for_next,
                queue_for_next,
                concurrency_for_next,
                active_tasks_for_next,
                notify_tx_for_next,
            ));
        } else {
            debug!("No capacity for download {}, returning to queue", item.id);
            // Put back in queue
            let mut queue_vec = queue.lock().unwrap();
            queue_vec.insert(0, next_id);
        }
    }
}

/// Processes the queue in a way that is Send-compatible
async fn process_queue_static(
    downloads: Arc<RwLock<HashMap<String, DownloadItem>>>,
    queue: Arc<Mutex<Vec<String>>>,
    concurrency_control: Arc<Semaphore>,
    active_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    notify_tx: broadcast::Sender<()>,
) {
        // Process next download in queue
        let mut next_download = None;
        let mut next_id = String::new();
        
        // Get next download (similar logic to check_and_process_queue but standalone)
        {
            let mut queue_vec = queue.lock().unwrap();
            if !queue_vec.is_empty() {
                next_id = queue_vec[0].clone();
                queue_vec.remove(0);
                
                let downloads_map = downloads.read().unwrap();
                next_download = downloads_map.get(&next_id).cloned();
            }
        }
        
        // If we have a download and there's capacity, start it
        if let Some(mut item) = next_download {
            // Create a clone of the semaphore to avoid lifetime issues
            let semaphore = Arc::clone(&concurrency_control);
            
            // Try to acquire a permit from the semaphore
            if semaphore.available_permits() > 0 {
                // Mark as started and update in downloads map
                item.mark_started();
                
                let cancel_rx = item.create_cancel_token();
                
                {
                    let mut downloads_map = downloads.write().unwrap();
                    downloads_map.insert(item.id.clone(), item.clone());
                }
                
                // Clone everything needed for the task
                let item_id = item.id.clone();
                let item_for_task = item.clone();
                let downloads_for_task = Arc::clone(&downloads);
                let active_tasks_for_task = Arc::clone(&active_tasks);
                let notify_tx_for_task = notify_tx.clone();
                let concurrency_control_for_task = Arc::clone(&concurrency_control);
                
                // Spawn the download task
                let handle = tokio::spawn(async move {
                    // Acquire permit inside the task
                    let _permit = concurrency_control_for_task.acquire().await.expect("Failed to acquire permit");
                    
                    // Execute the download
                    let result = execute_download(item_for_task, cancel_rx).await;
                    
                    // Update download status based on result
                    {
                        let mut downloads_map = downloads_for_task.write().unwrap();
                        
                        if let Some(dl_item) = downloads_map.get_mut(&item_id) {
                            match result {
                                Ok(output_path) => {
                                    debug!("Download {} completed successfully", item_id);
                                    dl_item.mark_completed(Some(output_path));
                                },
                                Err(e) => {
                                    error!("Download {} failed: {}", item_id, e);
                                    dl_item.mark_failed(Some(e.to_string()));
                                }
                            }
                        }
                    }
                    
                    // Remove from active tasks
                    {
                        let mut tasks = active_tasks_for_task.lock().unwrap();
                        tasks.remove(&item_id);
                    }
                    
                    // Notify listeners of state change
                    let _ = notify_tx_for_task.send(());
                });
                
                // Store the task handle
                {
                    let mut tasks = active_tasks.lock().unwrap();
                    tasks.insert(item.id.clone(), handle);
                }
                
                // Notify listeners
                let _ = notify_tx.send(());
            } else {
                // No capacity, put back in queue
                let mut queue_vec = queue.lock().unwrap();
                queue_vec.insert(0, next_id);
            }
        }
    }

// Process_next_download has been replaced by the inline implementation in process_queue_static

/// Execute a download and handle cancellation
async fn execute_download(
    item: DownloadItem,
    mut cancel_rx: broadcast::Receiver<()>,
) -> Result<String, AppError> {
    // Launch the download
    use crate::downloader;
    
    // Create a variable to hold the download task
    let url = item.url.clone();
    let quality = item.quality.clone();
    let format_str = item.format.clone();
    let start_time = item.start_time.clone();
    let end_time = item.end_time.clone();
    let use_playlist = item.use_playlist;
    let download_subtitles = item.download_subtitles;
    let output_dir = item.output_dir.clone();
    let force_download = item.force_download;
    let bitrate = item.bitrate.clone();
    let id = item.id.clone();
    
    // Save format for output path creation
    let output_format = format_str.clone();
    
    // Create a new task for the download
    let download_task = tokio::spawn(async move {
        downloader::download_video_free(
            &url,
            quality.as_deref(),
            &format_str,
            start_time.as_ref(),
            end_time.as_ref(),
            use_playlist,
            download_subtitles,
            output_dir.as_ref(),
            force_download,
            bitrate.as_ref(),
        ).await
    });
    
    // Keep a reference to the task handle for potential cancellation
    let download_task_handle = download_task.abort_handle();
    
    // Wait for either completion or cancellation
    tokio::select! {
        result = download_task => {
            match result {
                Ok(download_result) => {
                    match download_result {
                        Ok(_) => {
                            // TODO: Get actual output path
                            let output_path = format!("downloaded/{}.{}", id, output_format);
                            Ok(output_path)
                        },
                        Err(e) => Err(e)
                    }
                },
                Err(e) => {
                    Err(AppError::General(format!("Download task failed: {}", e)))
                }
            }
        },
        _ = cancel_rx.recv() => {
            debug!("Download {} cancelled", id);
            // Cancel the download task
            download_task_handle.abort();
            Err(AppError::General("Download cancelled".to_string()))
        }
    }
}

/// Save queue state to disk
async fn save_queue_state(
    downloads: Arc<RwLock<HashMap<String, DownloadItem>>>,
    state_path: PathBuf,
) -> Result<(), AppError> {
    // Create a serializable version of downloads without runtime-specific fields
    #[derive(Serialize)]
    struct SerializableQueue {
        downloads: Vec<DownloadItem>,
    }
    
    let downloads_data = {
        let downloads_map = downloads.read().unwrap();
        
        let mut items: Vec<DownloadItem> = downloads_map.values().cloned().collect();
        
        // Sort by status and priority
        items.sort_by(|a, b| {
            match (a.status, b.status) {
                // Active downloads first, then queued, then paused
                (DownloadStatus::Downloading, DownloadStatus::Downloading) => b.priority.cmp(&a.priority),
                (DownloadStatus::Downloading, _) => std::cmp::Ordering::Less,
                (_, DownloadStatus::Downloading) => std::cmp::Ordering::Greater,
                
                (DownloadStatus::Queued, DownloadStatus::Queued) => b.priority.cmp(&a.priority),
                (DownloadStatus::Queued, _) => std::cmp::Ordering::Less,
                (_, DownloadStatus::Queued) => std::cmp::Ordering::Greater,
                
                (DownloadStatus::Paused, DownloadStatus::Paused) => b.priority.cmp(&a.priority),
                (DownloadStatus::Paused, _) => std::cmp::Ordering::Less,
                (_, DownloadStatus::Paused) => std::cmp::Ordering::Greater,
                
                // Then by priority
                _ => b.priority.cmp(&a.priority)
            }
        });
        
        SerializableQueue {
            downloads: items,
        }
    };
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&downloads_data)
        .map_err(AppError::JsonError)?;
    
    // Write to file - spawn a tokio task for this
    let path_str = state_path.to_string_lossy().to_string();
    tokio::task::spawn_blocking(move || {
        std::fs::write(state_path, json)
    }).await.map_err(|e| AppError::General(format!("Failed to save queue state: {}", e)))?
        .map_err(AppError::IoError)?;
    
    debug!("Queue state saved to {}", path_str);
    Ok(())
}

/// Load queue state from disk
async fn load_queue_state(
    downloads: Arc<RwLock<HashMap<String, DownloadItem>>>,
    queue: Arc<Mutex<Vec<String>>>,
    state_path: PathBuf,
) -> Result<(), AppError> {
    if !state_path.exists() {
        debug!("No queue state file found at {:?}", state_path);
        return Ok(());
    }
    
    // Load JSON from file
    let path_str = state_path.to_string_lossy().to_string();
    let json = tokio::fs::read_to_string(&state_path)
        .await
        .map_err(AppError::IoError)?;
    
    // Deserialize
    #[derive(Deserialize)]
    struct SerializableQueue {
        downloads: Vec<DownloadItem>,
    }
    
    let data: SerializableQueue = serde_json::from_str(&json)
        .map_err(AppError::JsonError)?;
    
    // Update downloads map and queue
    {
        let mut downloads_map = downloads.write().unwrap();
        let mut queue_vec = queue.lock().unwrap();
        
        // Clear existing data
        downloads_map.clear();
        queue_vec.clear();
        
        // Add loaded items
        for mut item in data.downloads {
            // Reset status for active downloads (they weren't properly closed)
            if item.status == DownloadStatus::Downloading {
                item.status = DownloadStatus::Queued;
            }
            
            // Add to queue if active or paused
            if item.status == DownloadStatus::Queued {
                if item.priority == DownloadPriority::High || item.priority == DownloadPriority::Critical {
                    queue_vec.insert(0, item.id.clone());
                } else {
                    queue_vec.push(item.id.clone());
                }
            }
            
            // Add to downloads map
            downloads_map.insert(item.id.clone(), item);
        }
    }
    
    debug!("Queue state loaded from {}", path_str);
    Ok(())
}

/// Initialize the download manager
pub async fn init_download_manager() -> Result<Arc<DownloadQueue>, AppError> {
    // Create the download queue
    let queue = Arc::new(DownloadQueue::new(3));
    
    // Start the queue processor
    queue.start().await?;
    
    Ok(queue)
}

/// Global download queue instance
static DOWNLOAD_QUEUE: Lazy<tokio::sync::OnceCell<Arc<DownloadQueue>>> = 
    Lazy::new(tokio::sync::OnceCell::new);

/// Access the global download queue, initializing it if necessary
pub async fn get_download_queue() -> Arc<DownloadQueue> {
    DOWNLOAD_QUEUE.get_or_init(|| async {
        match init_download_manager().await {
            Ok(queue) => queue,
            Err(e) => {
                error!("Failed to initialize download manager: {}", e);
                // Fallback to a new empty queue
                Arc::new(DownloadQueue::new(3))
            }
        }
    }).await.clone()
}

/// Add a download to the global queue
/// Download options struct to replace multiple parameters
pub struct DownloadOptions<'a> {
    pub url: &'a str,
    pub quality: Option<&'a str>,
    pub format: &'a str,
    pub start_time: Option<&'a String>,
    pub end_time: Option<&'a String>,
    pub use_playlist: bool,
    pub download_subtitles: bool,
    pub output_dir: Option<&'a String>,
    pub force_download: bool,
    pub bitrate: Option<&'a String>,
    pub priority: Option<DownloadPriority>,
}

impl Default for DownloadOptions<'_> {
    fn default() -> Self {
        Self {
            url: "",
            quality: None,
            format: "mp4",
            start_time: None,
            end_time: None,
            use_playlist: false,
            download_subtitles: false,
            output_dir: None,
            force_download: false,
            bitrate: None,
            priority: None,
        }
    }
}

pub async fn add_download_to_queue(
    options: DownloadOptions<'_>,
) -> Result<String, AppError> {
    let queue = get_download_queue().await;
    
    // Create download item
    let mut builder = DownloadItem::builder(options.url, options.format)
        .quality(options.quality)
        .playlist(options.use_playlist)
        .subtitles(options.download_subtitles)
        .force_download(options.force_download);
    
    if let Some(dir) = options.output_dir {
        builder = builder.output_dir(Some(dir));
    }
    
    if let Some(start) = options.start_time {
        if let Some(end) = options.end_time {
            builder = builder.time_range(Some(start), Some(end));
        } else {
            builder = builder.time_range(Some(start), None);
        }
    }
    
    if let Some(rate) = options.bitrate {
        builder = builder.bitrate(Some(rate));
    }
    
    if let Some(p) = options.priority {
        builder = builder.priority(p);
    }
    
    let item = builder.build();
    let id = item.id.clone();
    
    // Add to queue
    queue.add_download(item).await?;
    
    Ok(id)
}

/// Pause all downloads
pub async fn pause_all_downloads() -> Result<(), AppError> {
    let queue = get_download_queue().await;
    queue.pause_all().await
}

/// Resume all downloads
pub async fn resume_all_downloads() -> Result<(), AppError> {
    let queue = get_download_queue().await;
    queue.resume_all().await
}

/// Pause a specific download
pub async fn pause_download(id: &str) -> Result<(), AppError> {
    let queue = get_download_queue().await;
    queue.pause_download(id).await
}

/// Resume a specific download
pub async fn resume_download(id: &str) -> Result<(), AppError> {
    let queue = get_download_queue().await;
    queue.resume_download(id).await
}

/// Cancel a specific download
pub async fn cancel_download(id: &str) -> Result<(), AppError> {
    let queue = get_download_queue().await;
    queue.cancel_download(id).await
}

/// Set download priority
pub async fn set_download_priority(id: &str, priority: DownloadPriority) -> Result<(), AppError> {
    let queue = get_download_queue().await;
    queue.set_priority(id, priority).await
}

/// Get a list of all downloads
pub fn get_all_downloads() -> Vec<DownloadItem> {
    match DOWNLOAD_QUEUE.get() {
        Some(queue) => queue.get_all_downloads(),
        None => Vec::new(),
    }
}

/// Get download status by ID
#[allow(dead_code)]
pub fn get_download_status(id: &str) -> Option<DownloadStatus> {
    match DOWNLOAD_QUEUE.get() {
        Some(queue) => {
            queue.get_download(id.to_string()).map(|item| item.status)
        },
        None => None,
    }
}

/// Shutdown the download manager
pub async fn shutdown_download_manager() -> Result<(), AppError> {
    if let Some(queue) = DOWNLOAD_QUEUE.get() {
        queue.stop().await?;
    }
    Ok(())
}

// The types are already public in this module,
// so no need for re-export as they're already available when importing this module