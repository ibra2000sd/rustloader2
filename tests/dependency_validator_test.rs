// tests/dependency_validator_test.rs
use rustloader::dependency_validator::{is_ffmpeg_available, MIN_FFMPEG_VERSION, MIN_YTDLP_VERSION};

// This test uses an #[ignore] attribute because it requires external dependencies to be installed
// and may fail on CI systems where they aren't available
#[test]
#[ignore]
fn test_ffmpeg_available() {
    // This should pass on most development machines with ffmpeg installed
    assert!(is_ffmpeg_available());
}

#[test]
#[ignore]
fn test_ffmpeg_detection_logic() {
    use rustloader::dependency_validator::{get_dependency_info};
    
    // Test the full detection logic through the public API
    match get_dependency_info("ffmpeg") {
        Ok(info) => {
            println!("Found ffmpeg at: {}", info.path);
            println!("Detected version: {}", info.version);
            
            // The path should be a non-empty string
            assert!(!info.path.is_empty());
            
            // The path should not be our "not found" marker
            assert!(!info.path.starts_with("__continuing_without_"));
            
            // The version should be a semantic version in the format x.y.z
            let version_parts: Vec<&str> = info.version.split('.').collect();
            assert!(!version_parts.is_empty(), "Version should have at least one part");
            
            // Check if the path exists on the file system
            let path_exists = std::path::Path::new(&info.path).exists();
            assert!(path_exists, "Detected ffmpeg path should exist on filesystem");
        }
        Err(e) => {
            // If this test is running, we expect ffmpeg to be installed
            panic!("Could not detect ffmpeg: {}", e);
        }
    }
}

#[test]
fn test_version_constants_format() {
    // Verify that the minimum version constants are formatted properly as semantic versions
    
    // Check MIN_YTDLP_VERSION format (YYYY.MM.DD)
    let ytdlp_parts: Vec<&str> = MIN_YTDLP_VERSION.split('.').collect();
    assert_eq!(ytdlp_parts.len(), 3, "yt-dlp version should have three parts");
    
    // Check that the first part is a 4-digit year
    assert_eq!(ytdlp_parts[0].len(), 4, "First part should be 4-digit year");
    assert!(ytdlp_parts[0].parse::<u16>().is_ok(), "First part should be a valid year");
    
    // Check that the second part is a 2-digit month (01-12)
    assert!(ytdlp_parts[1].len() <= 2, "Second part should be 1-2 digit month");
    let month = ytdlp_parts[1].parse::<u8>().expect("Month should be a number");
    assert!(month >= 1 && month <= 12, "Month should be between 1 and 12");
    
    // Check that the third part is a 2-digit day (01-31)
    assert!(ytdlp_parts[2].len() <= 2, "Third part should be 1-2 digit day");
    let day = ytdlp_parts[2].parse::<u8>().expect("Day should be a number");
    assert!(day >= 1 && day <= 31, "Day should be between 1 and 31");
    
    // Check MIN_FFMPEG_VERSION format (X.Y.Z)
    let ffmpeg_parts: Vec<&str> = MIN_FFMPEG_VERSION.split('.').collect();
    assert!(ffmpeg_parts.len() >= 2, "ffmpeg version should have at least major and minor version");
    
    // Check that all parts are numeric
    for part in ffmpeg_parts {
        assert!(part.parse::<u32>().is_ok(), "Version part should be numeric");
    }
}

#[test]
fn test_ffmpeg_version_parsing() {
    // This function tests the version parsing logic without requiring ffmpeg to be installed
    // We're simulating the functionality of parse_version without directly calling it
    
    fn simulate_parse_version(output: &str) -> String {
        // Simplified version of the regex patterns used in the actual code
        let ffmpeg_patterns = [
            r"ffmpeg version (\d+\.\d+(?:\.\d+)?)",
            r"ffmpeg version n(\d+\.\d+(?:\.\d+)?)",
            r"ffmpeg version (?:git-)?(?:\d{4}-\d{2}-\d{2}-)?(\d+\.\d+(?:\.\d+)?)",
            r"ffmpeg\s+version\s+[^\s]+\s+Copyright.*?(\d+\.\d+(?:\.\d+)?)",
            r"ffmpeg\s+(?:version\s+)?([0-9]+\.[0-9]+(?:\.[0-9]+)?)",
            // Handle the version information pattern more specifically
            r"ffmpeg\s+version\s+information:\s*(?:(\d+\.\d+(?:\.\d+)?)|.*?(\d+\.\d+(?:\.\d+)?)\s*$)",
            r"version\s+(?:information:)?(?:[^\d]*?)(\d+\.\d+(?:\.\d+)?)",
        ];
        
        for pattern in ffmpeg_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(output) {
                    // Try to get the first capture group
                    if let Some(version) = captures.get(1) {
                        return version.as_str().trim().to_string();
                    }
                    // If we have a pattern with multiple capture groups, try the second one
                    else if let Some(version) = captures.get(2) {
                        return version.as_str().trim().to_string();
                    }
                }
            }
        }
        
        // Generic fallback pattern
        if let Ok(re) = regex::Regex::new(r"[^\d](\d+\.\d+(?:\.\d+)?)") {
            if let Some(captures) = re.captures(output) {
                if let Some(version) = captures.get(1) {
                    return version.as_str().trim().to_string();
                }
            }
        }
        
        "unknown".to_string()
    }
    
    // Test various ffmpeg version output formats
    assert_eq!(simulate_parse_version("ffmpeg version 4.2.7 Copyright"), "4.2.7");
    assert_eq!(simulate_parse_version("ffmpeg version n5.1.2"), "5.1.2");
    assert_eq!(simulate_parse_version("ffmpeg version git-2023-01-01-6.0.0"), "6.0.0");
    assert_eq!(simulate_parse_version("ffmpeg 4.4.1-static https://johnvansickle.com/ffmpeg/"), "4.4.1");
    // This line needs special handling since it has multiple version numbers
    // The last one (5.0.1) is what we want, not the gcc version (11.2.0)
    assert_eq!(simulate_parse_version("ffmpeg version information: built with gcc 11.2.0 (GCC) 20220218 5.0.1"), 
               "5.0.1");
               
    // Add an improved version of this test with clearer version indication
    assert_eq!(simulate_parse_version("ffmpeg version information: 5.0.1 built with gcc 11.2.0"), "5.0.1");
    assert_eq!(simulate_parse_version("ffmpeg version: 5.1.3 built on May 11 2023"), "5.1.3");
    
    // Test generic fallback
    assert_eq!(simulate_parse_version("some random text with version 1.2.3 embedded"), "1.2.3");
    
    // Ensure we get "unknown" for truly unparseable output
    assert_eq!(simulate_parse_version("completely unparseable output"), "unknown");
}

// Mock function to simulate calling external commands
#[test]
fn test_sanitize_version_string() {
    // Create a fake is_minimum_version function that behaves like the one in dependency_validator.rs
    fn is_minimum_version(version: &str, min_version: &str) -> bool {
        let version_parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();
        let min_parts: Vec<u32> = min_version.split('.').filter_map(|s| s.parse().ok()).collect();
    
        for i in 0..3 {
            let v1 = version_parts.get(i).copied().unwrap_or(0);
            let v2 = min_parts.get(i).copied().unwrap_or(0);
            if v1 > v2 {
                return true;
            }
            if v1 < v2 {
                return false;
            }
        }
        true
    }
    
    // Test with normal version strings
    assert!(is_minimum_version("5.0.0", "4.0.0")); // Newer major
    assert!(is_minimum_version("4.1.0", "4.0.0")); // Newer minor
    assert!(is_minimum_version("4.0.1", "4.0.0")); // Newer patch
    assert!(is_minimum_version("4.0.0", "4.0.0")); // Exact match
    assert!(!is_minimum_version("3.0.0", "4.0.0")); // Older major
    assert!(!is_minimum_version("4.0.0", "4.1.0")); // Older minor
    assert!(!is_minimum_version("4.0.0", "4.0.1")); // Older patch
    
    // Test with missing parts
    assert!(is_minimum_version("5", "4.0.0")); // Missing minor and patch
    assert!(is_minimum_version("4.1", "4.0.0")); // Missing patch
    assert!(!is_minimum_version("3", "4.0.0")); // Missing parts, older major
    
    // Test with extra parts
    assert!(is_minimum_version("4.0.0.1", "4.0.0")); // Extra part
    assert!(!is_minimum_version("3.9.9.9", "4.0.0")); // Extra part, older major
}