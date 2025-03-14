//! Security configuration and utilities module for Rustloader
//!
//! This module provides centralized security settings, validation functions,
//! and utilities to enhance the overall security posture of the application.

use crate::error::AppError;
use std::path::Path;
use std::time::{Duration, Instant};
use ring::hmac;
use ring::rand::{SystemRandom, SecureRandom};
use base64::{Engine as _, engine::general_purpose};
use std::sync::{Once, Mutex};
use once_cell::sync::Lazy;
use std::collections::HashMap;

// Security configuration constants
#[allow(dead_code)]
pub const MAX_DAILY_DOWNLOADS: u32 = 5;  // Maximum daily downloads for free version
#[allow(dead_code)]
pub const ACTIVATION_MAX_ATTEMPTS: usize = 5;  // Maximum license activation attempts
#[allow(dead_code)]
pub const ACTIVATION_LOCKOUT_DURATION: Duration = Duration::from_secs(1800);  // 30 minutes
#[allow(dead_code)]
pub const HASH_ITERATIONS: u32 = 10000;  // PBKDF2 iterations for key derivation

// Sensitive directory patterns to avoid in path traversal checks
pub const SENSITIVE_DIRECTORIES: [&str; 12] = [
    "/etc", "/bin", "/sbin", "/usr/bin", "/usr/sbin",
    "/usr/local/bin", "/usr/local/sbin", "/var/run",
    "/boot", "/dev", "/proc", "/sys"
];

// Initialize the secure random number generator and rate limits using once_cell
#[allow(dead_code)]
static SECURE_RNG: Lazy<SystemRandom> = Lazy::new(|| SystemRandom::new());
static RATE_LIMITS: Lazy<Mutex<HashMap<String, Vec<Instant>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Initialization flag for security module
static INIT: Once = Once::new();

/// Initialize the security module
pub fn init() {
    INIT.call_once(|| {
        // Perform one-time security initialization
        
        // Verify integrity of security-critical files
        if let Err(e) = verify_application_integrity() {
            eprintln!("WARNING: Application integrity check failed: {}", e);
        }
        
        // Set secure process limits (where available)
        #[cfg(unix)]
        if let Err(e) = set_process_limits() {
            eprintln!("WARNING: Failed to set process limits: {}", e);
        }
    });
}

/// Generate a random secure token of specified length
#[allow(dead_code)]
pub fn generate_secure_token(length: usize) -> Result<String, AppError> {
    let mut bytes = vec![0u8; length];
    SECURE_RNG.fill(&mut bytes)
        .map_err(|_| AppError::SecurityViolation)?;
    
    Ok(general_purpose::STANDARD.encode(&bytes))
}

/// Apply rate limiting to security-sensitive operations
/// Returns true if the operation is allowed, false if rate limited
pub fn apply_rate_limit(operation: &str, max_attempts: usize, window: Duration) -> bool {
    let now = Instant::now();
    let mut limits = RATE_LIMITS.lock().unwrap();
    
    // Get or create the entry for this operation
    let attempts = limits.entry(operation.to_string()).or_insert_with(Vec::new);
    
    // Remove attempts outside the time window
    attempts.retain(|time| now.duration_since(*time) < window);
    
    // Check if we've exceeded the limit
    if attempts.len() >= max_attempts {
        return false;
    }
    
    // Add this attempt
    attempts.push(now);
    true
}

/// Generate an HMAC signature for the provided data
#[allow(dead_code)]
pub fn generate_hmac_signature(data: &[u8], key: &[u8]) -> Result<Vec<u8>, AppError> {
    let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    let signature = hmac::sign(&hmac_key, data);
    Ok(signature.as_ref().to_vec())
}

/// Verify an HMAC signature for the provided data
#[allow(dead_code)]
pub fn verify_hmac_signature(data: &[u8], signature: &[u8], key: &[u8]) -> Result<bool, AppError> {
    let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    
    match hmac::verify(&hmac_key, data, signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Enhanced path safety validation with centralized security settings
pub fn validate_path_safety(path: &Path) -> Result<(), AppError> {
    // Canonicalize the path to resolve any .. or symlinks
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If path doesn't exist yet, we need to check its components
            return check_path_components(path);
        }
    };
    
    // Get user's home directory for comparison
    let home_dir = match dirs_next::home_dir() {
        Some(dir) => dir,
        None => return Err(AppError::PathError("Could not determine home directory".to_string())),
    };
    
    // Get the canonical form of the home directory
    let canonical_home = match home_dir.canonicalize() {
        Ok(h) => h,
        Err(_) => return Err(AppError::PathError("Could not canonicalize home directory".to_string())),
    };
    
    // Get download directory (should be under home)
    let mut downloads_dir = home_dir.clone();
    downloads_dir.push("Downloads");
    
    // Convert to string for easier comparison
    let path_str = canonical_path.to_string_lossy().to_string();
    
    // Check if path is within allowed directories
    let allowed_paths = [
        canonical_home.to_string_lossy().to_string(),
        "/tmp".to_string(),
        "/var/tmp".to_string(),
    ];
    
    let in_allowed_path = allowed_paths.iter().any(|allowed| path_str.starts_with(allowed));
    
    if !in_allowed_path {
        return Err(AppError::SecurityViolation);
    }
    
    // Check if path contains any sensitive directories
    for dir in SENSITIVE_DIRECTORIES.iter() {
        if path_str.starts_with(dir) {
            return Err(AppError::SecurityViolation);
        }
    }
    
    Ok(())
}

/// Check path components for relative traversal attempts
fn check_path_components(path: &Path) -> Result<(), AppError> {
    let path_str = path.to_string_lossy();
    
    // Check for potential path traversal sequences
    if path_str.contains("../") || path_str.contains("..\\") || 
       path_str.contains("/..") || path_str.contains("\\..") ||
       path_str.contains("~") {
        return Err(AppError::SecurityViolation);
    }
    
    // Check each component
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                // Attempting to navigate up - potential path traversal
                return Err(AppError::SecurityViolation);
            },
            _ => continue,
        }
    }
    
    Ok(())
}

/// Verify the integrity of security-critical files
fn verify_application_integrity() -> Result<(), AppError> {
    // In a real implementation, this would verify hashes of critical files
    // against known good values
    
    // For demonstration purposes, just check if the binary exists
    let exe_path = std::env::current_exe()
        .map_err(|e| AppError::IoError(e))?;
    
    if !exe_path.exists() {
        return Err(AppError::SecurityViolation);
    }
    
    Ok(())
}

/// Set secure process limits (Unix-only)
#[cfg(unix)]
fn set_process_limits() -> Result<(), AppError> {
    use std::process::Command;
    
    // Example: Set resource limits using ulimit
    // This is platform-specific and just an example
    Command::new("ulimit")
        .arg("-n")
        .arg("1024")  // File descriptor limit
        .status()
        .map_err(|e| AppError::IoError(e))?;
    
    Ok(())
}

#[cfg(not(unix))]
fn set_process_limits() -> Result<(), AppError> {
    // No-op for non-Unix platforms
    Ok(())
}

/// Sanitize a string to make it safe for command-line use
#[allow(dead_code)]
pub fn sanitize_command_arg(arg: &str) -> Result<String, AppError> {
    // Define specific allowlists for different argument types
    
    // For bitrate arguments (e.g., 1000K)
    if arg.ends_with('K') || arg.ends_with('M') {
        let num_part = &arg[0..arg.len()-1];
        if num_part.chars().all(|c| c.is_ascii_digit()) {
            return Ok(arg.to_string());
        }
    }
    
    // For time arguments (e.g., 00:01:30)
    if arg.len() == 8 && arg.chars().nth(2) == Some(':') && arg.chars().nth(5) == Some(':') {
        let time_parts: Vec<&str> = arg.split(':').collect();
        if time_parts.len() == 3 && 
           time_parts.iter().all(|part| part.chars().all(|c| c.is_ascii_digit())) {
            return Ok(arg.to_string());
        }
    }
    
    // For format arguments (mp3, mp4, etc.)
    if ["mp3", "mp4", "webm", "m4a", "flac", "wav", "ogg"].contains(&arg) {
        return Ok(arg.to_string());
    }
    
    // For quality specifiers
    if ["480", "720", "1080", "2160", "best", "bestaudio"].contains(&arg) ||
       arg.starts_with("best[") && arg.ends_with("]") {
        return Ok(arg.to_string());
    }
    
    // For URLs - apply URL validation
    if arg.starts_with("http://") || arg.starts_with("https://") {
        // In a real implementation, you would call validate_url from utils.rs here
        // For now, we'll just do a basic check
        if arg.contains("&") || arg.contains(";") || arg.contains("|") {
            return Err(AppError::SecurityViolation);
        }
        return Ok(arg.to_string());
    }
    
    // For paths - validate separately
    if arg.contains('/') || arg.contains('\\') {
        let path = std::path::Path::new(arg);
        validate_path_safety(path)?;
        return Ok(arg.to_string());
    }
    
    // General whitelist for other arguments
    let valid_chars = arg.chars().all(|c| 
        c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-' || 
        c == '.' || c == ':' || c == '=' || c == '[' || c == ']'
    );
    
    if !valid_chars {
        return Err(AppError::ValidationError(format!("Invalid characters in argument: {}", arg)));
    }
    
    Ok(arg.to_string())
}

/// Check for potential command injection patterns
pub fn detect_command_injection(input: &str) -> bool {
    // Look for common command injection patterns
    let suspicious_patterns = [
        ";", "&", "&&", "||", "|", "`", "$(",
        "$()", ">${", ">%", "<${", "<%", "}}%", "$[",
    ];
    
    for pattern in suspicious_patterns.iter() {
        if input.contains(pattern) {
            return true; // Suspicious pattern found
        }
    }
    
    // Check for attempted escaping of quotes
    if input.contains("\\\"") || input.contains("\\'") {
        return true;
    }
    
    // Check for environment variable access
    if input.contains("$") && (input.contains("{") || input.contains("(")) {
        return true;
    }
    
    false // No suspicious patterns found
}

/// Validate URL format with security checks
#[allow(dead_code)]
pub fn validate_url(url: &str) -> Result<(), AppError> {
    // Basic URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::ValidationError("Invalid URL scheme".to_string()));
    }
    
    // Check for command injection attempts
    if detect_command_injection(url) {
        return Err(AppError::SecurityViolation);
    }
    
    // Check length to prevent DoS
    if url.len() > 2048 {
        return Err(AppError::ValidationError("URL is too long".to_string()));
    }
    
    // Ensure URL contains domain and TLD
    let domain_parts: Vec<&str> = url.split("://").collect();
    if domain_parts.len() != 2 || !domain_parts[1].contains('.') {
        return Err(AppError::ValidationError("Invalid URL format".to_string()));
    }
    
    Ok(())
}

/// Verify file integrity using a hash
#[allow(dead_code)]
pub fn verify_file_integrity(file_path: &Path, expected_hash: &str) -> Result<bool, AppError> {
    use std::fs::File;
    use std::io::Read;
    use ring::digest::{Context, SHA256};
    
    let mut file = File::open(file_path).map_err(|e| AppError::IoError(e))?;
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 8192];
    
    loop {
        let count = file.read(&mut buffer).map_err(|e| AppError::IoError(e))?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }
    
    let digest = context.finish();
    let hash = general_purpose::STANDARD_NO_PAD.encode(digest.as_ref());
    
    Ok(hash == expected_hash)
}

/// Secure deletion of sensitive files
#[allow(dead_code)]
pub fn secure_delete_file(file_path: &Path) -> Result<(), AppError> {
    use std::fs::{OpenOptions, remove_file};
    use std::io::{Write, Seek, SeekFrom};
    
    // Open the file for writing
    let mut file = OpenOptions::new()
        .write(true)
        .open(file_path)
        .map_err(|e| AppError::IoError(e))?;
    
    // Get file size
    let file_size = file.metadata()
        .map_err(|e| AppError::IoError(e))?
        .len() as usize;
    
    // Generate random data
    let mut buffer = vec![0u8; std::cmp::min(8192, file_size)];
    
    // Overwrite file with random data three times
    for _ in 0..3 {
        // Seek to beginning of file
        file.seek(SeekFrom::Start(0))
            .map_err(|e| AppError::IoError(e))?;
        
        // Fill file with random data
        let mut remaining = file_size;
        while remaining > 0 {
            let chunk_size = std::cmp::min(buffer.len(), remaining);
            SECURE_RNG.fill(&mut buffer[..chunk_size])
                .map_err(|_| AppError::SecurityViolation)?;
            
            file.write_all(&buffer[..chunk_size])
                .map_err(|e| AppError::IoError(e))?;
            
            remaining -= chunk_size;
        }
        
        // Flush to ensure write
        file.flush().map_err(|e| AppError::IoError(e))?;
    }
    
    // Close the file
    drop(file);
    
    // Remove the file
    remove_file(file_path).map_err(|e| AppError::IoError(e))?;
    
    Ok(())
}