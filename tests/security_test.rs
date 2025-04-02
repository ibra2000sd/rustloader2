// tests/security_test.rs
use rustloader::security::{apply_rate_limit, detect_command_injection};
// rustloader::error::AppError not directly used in this test
use std::path::Path;
use std::time::Duration;

#[test]
fn test_rate_limiting() {
    // Test operation should allow the first few attempts
    let operation = "test_operation";
    let max_attempts = 3;
    let window = Duration::from_secs(10);
    
    // First attempt should be allowed
    assert!(apply_rate_limit(operation, max_attempts, window));
    
    // Second attempt should be allowed
    assert!(apply_rate_limit(operation, max_attempts, window));
    
    // Third attempt should be allowed (limit is 3)
    assert!(apply_rate_limit(operation, max_attempts, window));
    
    // Fourth attempt should be denied
    assert!(!apply_rate_limit(operation, max_attempts, window));
}

#[test]
fn test_command_injection_detection() {
    // Clean inputs should pass
    assert!(!detect_command_injection("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
    assert!(!detect_command_injection("https://youtu.be/dQw4w9WgXcQ"));
    
    // Inputs with command injection patterns should be detected
    assert!(detect_command_injection("https://example.com/video?id=123;ls -la"));
    assert!(detect_command_injection("https://example.com/video?id=123`cat /etc/passwd`"));
    assert!(detect_command_injection("https://example.com/video?id=123$(cat /etc/passwd)"));
    assert!(detect_command_injection("https://example.com/video?id=123>${USER}"));
    assert!(detect_command_injection("https://example.com/video?id=123<${PATH}"));
    assert!(detect_command_injection("https://example.com/video?id=123\\\""));
    assert!(detect_command_injection("https://example.com/video?id=123\\'"));
    assert!(detect_command_injection("https://example.com/video?id=${HOME}"));
}

#[test]
fn test_path_safety_validation() {
    use rustloader::security::validate_path_safety;
    
    // Safe paths should be validated
    let home_dir = dirs_next::home_dir().unwrap();
    let download_dir = home_dir.join("Downloads");
    
    // Home directory should be considered safe
    assert!(validate_path_safety(&home_dir).is_ok());
    
    // Download directory should be considered safe
    assert!(validate_path_safety(&download_dir).is_ok());
    
    // Temporary directory should be considered safe
    if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
        assert!(validate_path_safety(&temp_dir).is_ok());
    }
    
    // Sensitive directories should be rejected
    #[cfg(unix)]
    {
        assert!(validate_path_safety(Path::new("/etc")).is_err());
        assert!(validate_path_safety(Path::new("/bin")).is_err());
        assert!(validate_path_safety(Path::new("/usr/bin")).is_err());
    }
    
    // Path traversal attempts should be rejected
    let traversal_path = home_dir.join("..").join("..").join("etc").join("passwd");
    assert!(validate_path_safety(&traversal_path).is_err());
}