// tests/utils_test.rs
use rustloader::utils::{validate_url, validate_time_format, validate_bitrate};

#[test]
fn test_validate_url_valid_formats() {
    // Valid URLs should pass validation
    assert!(validate_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").is_ok());
    assert!(validate_url("https://youtu.be/dQw4w9WgXcQ").is_ok());
    assert!(validate_url("https://vimeo.com/123456789").is_ok());
    assert!(validate_url("https://www.dailymotion.com/video/x123456").is_ok());
}

#[test]
fn test_validate_url_invalid_formats() {
    // Invalid URLs should fail validation
    assert!(validate_url("not-a-url").is_err());
    assert!(validate_url("file:///etc/passwd").is_err());
    assert!(validate_url("http://localhost:8080").is_err());
    assert!(validate_url("http://127.0.0.1").is_err());
    assert!(validate_url("https://example.com/test?cmd=`rm -rf /`").is_err());
    
    // URL with very long length should fail (potential DoS attack)
    let very_long_url = format!("https://example.com/{}", "a".repeat(5000));
    assert!(validate_url(&very_long_url).is_err());
}

#[test]
fn test_validate_time_format_valid() {
    // Valid time formats should pass validation
    assert!(validate_time_format("00:00:00").is_ok());
    assert!(validate_time_format("01:30:45").is_ok());
    assert!(validate_time_format("23:59:59").is_ok());
}

#[test]
fn test_validate_time_format_invalid() {
    // Invalid time formats should fail validation
    assert!(validate_time_format("0:0:0").is_err());
    assert!(validate_time_format("1:30:45").is_err());
    assert!(validate_time_format("24:00:00").is_err());
    assert!(validate_time_format("00:60:00").is_err());
    assert!(validate_time_format("00:00:60").is_err());
    assert!(validate_time_format("not-a-time").is_err());
    assert!(validate_time_format("00:00").is_err());
    assert!(validate_time_format("00:00:00:00").is_err());
}

#[test]
fn test_validate_bitrate_valid() {
    // Valid bitrate formats should pass validation
    assert!(validate_bitrate("1000K").is_ok());
    assert!(validate_bitrate("5M").is_ok());
    assert!(validate_bitrate("128K").is_ok());
    assert!(validate_bitrate("2M").is_ok());
}

#[test]
fn test_validate_bitrate_invalid() {
    // Invalid bitrate formats should fail validation
    assert!(validate_bitrate("1000").is_err());
    assert!(validate_bitrate("5G").is_err());
    assert!(validate_bitrate("not-a-bitrate").is_err());
    assert!(validate_bitrate("0K").is_err());
    assert!(validate_bitrate("12000K").is_err()); // Too high for K format
    assert!(validate_bitrate("200M").is_err());   // Too high for M format
}