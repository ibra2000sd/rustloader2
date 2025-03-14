// src/license.rs

use crate::error::AppError;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Local, Utc};
use dirs_next as dirs;
use ring::{digest, hmac};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// License information structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LicenseInfo {
    pub license_key: String,
    pub user_email: String,
    pub activation_date: DateTime<Utc>,
    pub expiration_date: Option<DateTime<Utc>>,
    pub machine_id: String,
}

// License verification result
pub enum LicenseStatus {
    Free,
    Pro(LicenseInfo),
    Invalid(String), // Contains the reason for invalidity
}

// Get a unique machine identifier
fn get_machine_id() -> Result<String, AppError> {
    #[cfg(target_os = "linux")]
    {
        // On Linux, try to use the machine-id
        match fs::read_to_string("/etc/machine-id") {
            Ok(id) => return Ok(id.trim().to_string()),
            Err(_) => {
                // Fallback to using hostname
                match hostname::get() {
                    Ok(name) => return Ok(name.to_string_lossy().to_string()),
                    Err(_) => {
                        return Err(AppError::General(
                            "Could not determine machine ID".to_string(),
                        ))
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, use the IOPlatformUUID
        use std::process::Command;

        let output = Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract the UUID using a simple search
        if let Some(line) = stdout.lines().find(|line| line.contains("IOPlatformUUID")) {
            if let Some(uuid_start) = line.find("\"") {
                if let Some(uuid_end) = line[uuid_start + 1..].find("\"") {
                    return Ok(line[uuid_start + 1..uuid_start + 1 + uuid_end].to_string());
                }
            }
        }

        // Fallback to hostname
        match hostname::get() {
            Ok(name) => return Ok(name.to_string_lossy().to_string()),
            Err(_) => {
                return Err(AppError::General(
                    "Could not determine machine ID".to_string(),
                ))
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, try to use the MachineGuid from registry
        use winreg::enums::*;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        match hklm.open_subkey("SOFTWARE\\Microsoft\\Cryptography") {
            Ok(key) => {
                match key.get_value::<String, _>("MachineGuid") {
                    Ok(guid) => return Ok(guid),
                    Err(_) => {
                        // Fallback to computer name on Windows if registry fails
                        match hostname::get() {
                            Ok(name) => return Ok(name.to_string_lossy().to_string()),
                            Err(_) => {
                                return Err(AppError::General(
                                    "Could not determine machine ID".to_string(),
                                ))
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback to computer name on Windows if registry access fails
                match hostname::get() {
                    Ok(name) => return Ok(name.to_string_lossy().to_string()),
                    Err(_) => {
                        return Err(AppError::General(
                            "Could not determine machine ID".to_string(),
                        ))
                    }
                }
            }
        }
    }

    // This code will ONLY run for non-Windows, non-Linux, non-macOS platforms
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // Fallback for other platforms - use hostname
        match hostname::get() {
            Ok(name) => return Ok(name.to_string_lossy().to_string()),
            Err(_) => {
                return Err(AppError::General(
                    "Could not determine machine ID".to_string(),
                ))
            }
        }
    }
}

// Obfuscated key generation with dynamic runtime component
fn get_verification_key() -> Vec<u8> {
    // Base components split across multiple variables to prevent easy extraction
    let base_component = [82, 117, 115, 116, 108, 111, 97, 100, 101, 114];
    let mid_component = [76, 105, 99, 101, 110, 115, 101];
    let end_component = [
        86, 101, 114, 105, 102, 105, 99, 97, 116, 105, 111, 110, 75, 101, 121,
    ];

    // Runtime component based on machine-specific attributes
    let dynamic_salt = match get_machine_id() {
        Ok(id) => {
            // Use a hash of the machine ID as a salt
            let digest = digest::digest(&digest::SHA256, id.as_bytes());
            let hash_bytes = digest.as_ref();
            // Use only a portion of the hash for the salt
            let mut salt = Vec::with_capacity(8);
            for i in 0..8 {
                salt.push(hash_bytes[i] ^ hash_bytes[i + 8]);
            }
            salt
        }
        Err(_) => {
            // Fallback salt if machine ID can't be determined
            vec![25, 62, 41, 53, 84, 29, 17, 36]
        }
    };

    // Combine all components with transformations
    let mut key = Vec::with_capacity(
        base_component.len() + mid_component.len() + end_component.len() + dynamic_salt.len(),
    );

    // Apply transformations during combination
    for b in base_component.iter() {
        key.push(*b);
    }

    for b in dynamic_salt.iter() {
        key.push(*b);
    }

    for b in mid_component.iter() {
        key.push(*b);
    }

    for (i, b) in end_component.iter().enumerate() {
        // XOR with a value derived from the index for additional obfuscation
        key.push(b ^ ((i as u8) % 7));
    }

    // Final transformation - hash the combined key to get the actual key
    let final_digest = digest::digest(&digest::SHA256, &key);
    final_digest.as_ref().to_vec()
}

// Path to the license file
fn get_license_path() -> Result<PathBuf, AppError> {
    let mut path = dirs::config_dir()
        .ok_or_else(|| AppError::PathError("Could not find config directory".to_string()))?;

    path.push("rustloader");
    fs::create_dir_all(&path)?;

    path.push("license.dat");
    Ok(path)
}

// Improved license verification with server check and additional validations
fn verify_license_with_server(license_key: &str) -> Result<bool, AppError> {
    // In a real implementation, this would make an HTTPS request to a license server
    // with proper TLS certificate validation

    // Basic offline validation improved
    if !license_key.starts_with("PRO-") || license_key.len() < 20 {
        return Ok(false);
    }

    // Check for valid license format with more strict requirements
    let valid_chars = license_key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

    if !valid_chars {
        return Ok(false);
    }

    // Check for expected segments in the license key
    let segments: Vec<&str> = license_key.split('-').collect();
    if segments.len() != 4 {
        return Ok(false);
    }

    // Verify the checksum segment (last segment)
    if let Some(checksum) = segments.last() {
        // Calculate expected checksum from other segments
        let data = segments[0..segments.len() - 1].join("-");
        let digest = digest::digest(&digest::SHA256, data.as_bytes());
        let expected_checksum = general_purpose::STANDARD.encode(&digest.as_ref()[0..6]);

        if **checksum != expected_checksum[0..8] {
            return Ok(false);
        }
    } else {
        return Ok(false);
    }

    // Add timestamp validation - licenses should be issued after program release
    let timestamp_segment = segments[2];
    if let Ok(timestamp) = timestamp_segment.parse::<u64>() {
        // Licenses should be issued after Jan 1, 2024 (timestamp 1704067200)
        const MIN_LICENSE_TIMESTAMP: u64 = 1704067200;
        if timestamp < MIN_LICENSE_TIMESTAMP {
            return Ok(false);
        }
    } else {
        return Ok(false);
    }

    Ok(true)
}

// Generate a signature for the license data
fn generate_license_signature(license: &LicenseInfo) -> Result<String, AppError> {
    let license_json = serde_json::to_string(license)?;

    let key = hmac::Key::new(hmac::HMAC_SHA256, &get_verification_key());
    let signature = hmac::sign(&key, license_json.as_bytes());

    Ok(general_purpose::STANDARD.encode(signature.as_ref()))
}

// Verify a license signature
fn verify_license_signature(license: &LicenseInfo, signature: &str) -> Result<bool, AppError> {
    let license_json = serde_json::to_string(license)?;

    let key = hmac::Key::new(hmac::HMAC_SHA256, &get_verification_key());

    let sig_bytes = match general_purpose::STANDARD.decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false),
    };

    match hmac::verify(&key, license_json.as_bytes(), &sig_bytes) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

// Save license information to disk
pub fn save_license(license: &LicenseInfo) -> Result<(), AppError> {
    let license_path = get_license_path()?;

    // Create a signature for the license data
    let signature = generate_license_signature(license)?;

    // Combine license data and signature
    let license_data = serde_json::to_string(license)?;
    let full_data = format!("{}\n{}", license_data, signature);

    // Encrypt or encode the data for additional security
    // For this example, we'll use simple base64 encoding
    let encoded_data = general_purpose::STANDARD.encode(full_data);

    // Write to file
    fs::write(license_path, encoded_data)?;

    Ok(())
}

// Load and verify license from disk
pub fn load_license() -> Result<LicenseStatus, AppError> {
    let license_path = get_license_path()?;

    // Check if license file exists
    if !license_path.exists() {
        return Ok(LicenseStatus::Free);
    }

    // Read and decode the license file
    let encoded_data = fs::read_to_string(license_path)?;
    let full_data = match general_purpose::STANDARD.decode(encoded_data) {
        Ok(data) => String::from_utf8(data)
            .map_err(|_| AppError::LicenseError("Invalid license data encoding".to_string()))?,
        Err(_) => {
            return Ok(LicenseStatus::Invalid(
                "License file is corrupted".to_string(),
            ))
        }
    };

    // Split into license data and signature
    let parts: Vec<&str> = full_data.split('\n').collect();
    if parts.len() != 2 {
        return Ok(LicenseStatus::Invalid(
            "Invalid license file format".to_string(),
        ));
    }

    let license_data = parts[0];
    let signature = parts[1];

    // Parse license data
    let license: LicenseInfo = match serde_json::from_str(license_data) {
        Ok(license) => license,
        Err(_) => {
            return Ok(LicenseStatus::Invalid(
                "License data is corrupted".to_string(),
            ))
        }
    };

    // Verify signature
    if !verify_license_signature(&license, signature)? {
        return Ok(LicenseStatus::Invalid(
            "License signature is invalid".to_string(),
        ));
    }

    // Check if license has expired
    if let Some(expiration) = license.expiration_date {
        if expiration < Utc::now() {
            return Ok(LicenseStatus::Invalid("License has expired".to_string()));
        }
    }

    // Verify machine ID matches
    let machine_id = get_machine_id()?;
    if license.machine_id != machine_id {
        return Ok(LicenseStatus::Invalid(
            "License is for a different machine".to_string(),
        ));
    }

    // Verify license with server (optional, can be disabled for offline use)
    if verify_license_with_server(&license.license_key)? {
        Ok(LicenseStatus::Pro(license))
    } else {
        Ok(LicenseStatus::Invalid(
            "License key is not valid".to_string(),
        ))
    }
}

// Check if the current installation is Pro
pub fn is_pro_version() -> bool {
    match load_license() {
        Ok(LicenseStatus::Pro(_)) => true,
        _ => false,
    }
}

// Activate a license key
pub fn activate_license(license_key: &str, email: &str) -> Result<LicenseStatus, AppError> {
    // Verify license with server
    if !verify_license_with_server(license_key)? {
        return Ok(LicenseStatus::Invalid("Invalid license key".to_string()));
    }

    // Create new license info
    let license = LicenseInfo {
        license_key: license_key.to_string(),
        user_email: email.to_string(),
        activation_date: Utc::now(),
        expiration_date: None, // Perpetual license for this example
        machine_id: get_machine_id()?,
    };

    // Save license to disk
    save_license(&license)?;

    Ok(LicenseStatus::Pro(license))
}

// Function to display license information
pub fn display_license_info() -> Result<(), AppError> {
    match load_license()? {
        LicenseStatus::Free => {
            println!("License: Free Version");
            println!("Upgrade to Pro: rustloader.com/pro");
        }
        LicenseStatus::Pro(license) => {
            println!("License: Pro Version");
            println!("Email: {}", license.user_email);
            println!(
                "Activated: {}",
                license
                    .activation_date
                    .with_timezone(&Local)
                    .format("%Y-%m-%d")
            );
            if let Some(exp) = license.expiration_date {
                println!("Expires: {}", exp.with_timezone(&Local).format("%Y-%m-%d"));
            } else {
                println!("Expires: Never (Perpetual License)");
            }
        }
        LicenseStatus::Invalid(reason) => {
            println!("License: Invalid");
            println!("Reason: {}", reason);
            println!("Reverting to Free Version");
        }
    }

    Ok(())
}
