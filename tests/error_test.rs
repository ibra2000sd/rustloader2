// tests/error_test.rs
use rustloader::error::AppError;
use std::io;

#[test]
fn test_app_error_display() {
    // Test that error messages are formatted correctly
    
    // Test MissingDependency error
    let error = AppError::MissingDependency("ffmpeg".to_string());
    assert_eq!(error.to_string(), "Missing dependency: ffmpeg");
    
    // Test DownloadError error
    let error = AppError::DownloadError("Failed to download file".to_string());
    assert_eq!(error.to_string(), "Download error: Failed to download file");
    
    // Test ValidationError error
    let error = AppError::ValidationError("Invalid URL".to_string());
    assert_eq!(error.to_string(), "Validation error: Invalid URL");
    
    // Test IoError error
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let error = AppError::IoError(io_error);
    assert_eq!(error.to_string(), "I/O error: File not found");
    
    // Test TimeFormatError error
    let error = AppError::TimeFormatError("Invalid time format".to_string());
    assert_eq!(error.to_string(), "Time format error: Invalid time format");
    
    // Test PathError error
    let error = AppError::PathError("Invalid path".to_string());
    assert_eq!(error.to_string(), "Path error: Invalid path");
    
    // Test General error
    let error = AppError::General("General error".to_string());
    assert_eq!(error.to_string(), "Application error: General error");
    
    // Test DailyLimitExceeded error
    let error = AppError::DailyLimitExceeded;
    assert_eq!(error.to_string(), "Daily download limit exceeded");
    
    // Test PremiumFeature error
    let error = AppError::PremiumFeature("HD downloads".to_string());
    assert_eq!(error.to_string(), "Premium feature: HD downloads");
    
    // Test SecurityViolation error
    let error = AppError::SecurityViolation;
    assert_eq!(
        error.to_string(),
        "Security violation detected. If this is unexpected, please report this issue."
    );
    
    // Test LicenseError error
    let error = AppError::LicenseError("Invalid license key".to_string());
    assert_eq!(error.to_string(), "License error: Invalid license key");
    
    // Test ParseError error
    let error = AppError::ParseError("Failed to parse config".to_string());
    assert_eq!(error.to_string(), "Parse error: Failed to parse config");
}

#[test]
fn test_from_string_for_app_error() {
    // Test conversion from String to AppError
    let error_string = "Test error".to_string();
    let error: AppError = error_string.into();
    
    match error {
        AppError::General(message) => assert_eq!(message, "Test error"),
        _ => panic!("Expected AppError::General"),
    }
}

#[test]
fn test_from_str_for_app_error() {
    // Test conversion from &str to AppError
    let error_str = "Test error";
    let error: AppError = error_str.into();
    
    match error {
        AppError::General(message) => assert_eq!(message, "Test error"),
        _ => panic!("Expected AppError::General"),
    }
}