use log::{debug, error};
use serde::Serialize;
use std::path::PathBuf;
use tauri::AppHandle;

// Constants for notification icons based on platform
const DEFAULT_ICON: &str = "icons/notification-icon.png";
const SUCCESS_ICON: Option<&str> = Some("icons/notification-success.png");
const ERROR_ICON: Option<&str> = Some("icons/notification-error.png");
const INFO_ICON: Option<&str> = Some("icons/notification-info.png");

#[derive(Debug, Serialize, Clone)]
pub enum NotificationType {
    Success,
    Error,
    Info,
    Default,
}

pub struct NotificationOptions {
    pub title: String,
    pub body: String,
    pub notification_type: NotificationType,
    pub silent: bool,
    pub icon: Option<String>,
}

impl Default for NotificationOptions {
    fn default() -> Self {
        Self {
            title: String::new(),
            body: String::new(),
            notification_type: NotificationType::Default,
            silent: false,
            icon: None,
        }
    }
}

/// Cross-platform notification utility
pub struct NotificationManager {
    app: AppHandle,
    // Track if notifications are enabled
    enabled: bool,
}

impl NotificationManager {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            enabled: true,
        }
    }

    /// Set notification enablement state
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get notification enablement state
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Send a notification with the provided options
    pub fn send_notification(&self, options: NotificationOptions) {
        // Don't send if notifications are disabled
        if !self.enabled {
            return;
        }

        // In Tauri 2.x we would typically use a notification plugin
        // For now, we'll create a simple fallback implementation
        debug!("Sending notification: {} - {}", options.title, options.body);
        
        // Using println for now; in a real implementation, you would use the Tauri notification plugin
        println!("NOTIFICATION: {} - {}", options.title, options.body);
        
        // Log successful notification
        debug!("Notification sent successfully");
    }

    /// Helper to send download completion notification
    pub fn send_download_complete(&self, title: &str, file_name: &str) {
        self.send_notification(NotificationOptions {
            title: title.to_string(),
            body: format!("Download complete: {}", file_name),
            notification_type: NotificationType::Success,
            silent: false,
            icon: None,
        });
    }

    /// Helper to send download error notification
    pub fn send_download_error(&self, title: &str, error_message: &str) {
        self.send_notification(NotificationOptions {
            title: title.to_string(),
            body: format!("Download failed: {}", error_message),
            notification_type: NotificationType::Error,
            silent: false,
            icon: None,
        });
    }

    /// Helper to send download started notification
    pub fn send_download_started(&self, title: &str, file_name: &str) {
        self.send_notification(NotificationOptions {
            title: title.to_string(),
            body: format!("Started downloading: {}", file_name),
            notification_type: NotificationType::Info,
            silent: true, // Silent for start notifications to avoid noise
            icon: None,
        });
    }

    // Helper method to resolve icon path based on notification type
    fn resolve_icon_path(&self, resource_path: &PathBuf, options: &NotificationOptions) -> PathBuf {
        // Use custom icon if provided
        if let Some(icon) = &options.icon {
            return resource_path.join(icon);
        }

        // Otherwise use type-specific icon
        let icon_name = match options.notification_type {
            NotificationType::Success => SUCCESS_ICON.unwrap_or(DEFAULT_ICON),
            NotificationType::Error => ERROR_ICON.unwrap_or(DEFAULT_ICON),
            NotificationType::Info => INFO_ICON.unwrap_or(DEFAULT_ICON),
            NotificationType::Default => DEFAULT_ICON,
        };

        resource_path.join(icon_name)
    }
}

/// Tauri command to check if notifications are supported
#[tauri::command]
pub fn are_notifications_supported() -> bool {
    // In Tauri 2.x, this would use a platform-specific check
    // For now, we'll assume notifications are supported
    true
}

/// Tauri command to request permission for notifications
#[tauri::command]
pub async fn request_notification_permission() -> bool {
    // In Tauri 2.x, this would use a platform-specific permission request
    // For now, we'll assume permission is granted
    true
}

/// Tauri command to toggle notifications
#[tauri::command]
pub fn toggle_notifications(enabled: bool, state: tauri::State<'_, NotificationState>) -> bool {
    let mut mgr = state.0.lock().unwrap();
    mgr.set_enabled(enabled);
    enabled
}

/// State wrapper for notification manager
pub struct NotificationState(pub std::sync::Mutex<NotificationManager>);