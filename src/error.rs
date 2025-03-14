// src/error.rs

use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeError;
use std::io;
use thiserror::Error;

/// Custom error types for the application
#[derive(Error, Debug)]
pub enum AppError {
    /// Error for missing dependencies
    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    /// Error during download process
    #[error("Download error: {0}")]
    DownloadError(String),

    /// Error for invalid input validation
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// I/O related errors
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// Error for invalid time format
    #[error("Time format error: {0}")]
    TimeFormatError(String),

    /// Error for path operation failures
    #[error("Path error: {0}")]
    PathError(String),

    /// General application errors
    #[error("Application error: {0}")]
    General(String),

    /// Error for when daily download limit is exceeded
    #[error("Daily download limit exceeded")]
    DailyLimitExceeded,

    /// Error for when a feature requires the Pro version
    #[error("Premium feature: {0}")]
    #[allow(dead_code)]
    PremiumFeature(String),

    /// Error for security violations (tampering, path traversal, etc.)
    #[error("Security violation detected. If this is unexpected, please report this issue.")]
    SecurityViolation,

    /// HTTP client errors
    #[error("HTTP error: {0}")]
    HttpError(#[from] ReqwestError),

    /// JSON parsing errors
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] SerdeError),

    /// License errors
    #[error("License error: {0}")]
    LicenseError(String),

    /// Parse errors
    #[allow(dead_code)]
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Convert a string error to AppError::General
impl From<String> for AppError {
    fn from(error: String) -> Self {
        AppError::General(error)
    }
}

/// Convert a &str error to AppError::General
impl From<&str> for AppError {
    fn from(error: &str) -> Self {
        AppError::General(error.to_string())
    }
}
