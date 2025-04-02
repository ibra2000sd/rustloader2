// tests/cli_test.rs
use rustloader::cli::build_cli;

#[test]
fn test_cli_basic_structure() {
    // Build the CLI
    let app = build_cli();
    
    // Check that the app has the expected name, version, and author
    assert_eq!(app.get_name(), "rustloader");
    
    // Check if basic arguments are present
    let matches = app.clone().try_get_matches_from(vec!["rustloader", "https://example.com"]);
    assert!(matches.is_ok());
    
    let matches = matches.unwrap();
    
    // Check that the URL argument was parsed correctly
    let url = matches.get_one::<String>("url").unwrap();
    assert_eq!(url, "https://example.com");
}

#[test]
fn test_cli_options() {
    let app = build_cli();
    
    // Test quality option
    let matches = app.clone()
        .try_get_matches_from(vec![
            "rustloader", 
            "https://example.com", 
            "--quality", 
            "720"
        ])
        .unwrap();
    
    let quality = matches.get_one::<String>("quality").unwrap();
    assert_eq!(quality, "720");
    
    // Test format option
    let matches = app.clone()
        .try_get_matches_from(vec![
            "rustloader", 
            "https://example.com", 
            "--format", 
            "mp3"
        ])
        .unwrap();
    
    let format = matches.get_one::<String>("format").unwrap();
    assert_eq!(format, "mp3");
    
    // Test start-time and end-time options
    let matches = app.clone()
        .try_get_matches_from(vec![
            "rustloader", 
            "https://example.com", 
            "--start-time", 
            "00:01:30",
            "--end-time",
            "00:02:45"
        ])
        .unwrap();
    
    let start_time = matches.get_one::<String>("start-time").unwrap();
    assert_eq!(start_time, "00:01:30");
    
    let end_time = matches.get_one::<String>("end-time").unwrap();
    assert_eq!(end_time, "00:02:45");
    
    // Test playlist flag
    let matches = app.clone()
        .try_get_matches_from(vec![
            "rustloader", 
            "https://example.com", 
            "--playlist"
        ])
        .unwrap();
    
    assert!(matches.get_flag("playlist"));
    
    // Test subtitles flag
    let matches = app.clone()
        .try_get_matches_from(vec![
            "rustloader", 
            "https://example.com", 
            "--subs"
        ])
        .unwrap();
    
    assert!(matches.get_flag("subtitles"));
    
    // Test output-dir option
    let matches = app.clone()
        .try_get_matches_from(vec![
            "rustloader", 
            "https://example.com", 
            "--output-dir", 
            "/tmp/downloads"
        ])
        .unwrap();
    
    let output_dir = matches.get_one::<String>("output-dir").unwrap();
    assert_eq!(output_dir, "/tmp/downloads");
    
    // Test video-bitrate option
    let matches = app.clone()
        .try_get_matches_from(vec![
            "rustloader", 
            "https://example.com", 
            "--bitrate", 
            "1000K"
        ])
        .unwrap();
    
    let bitrate = matches.get_one::<String>("video-bitrate").unwrap();
    assert_eq!(bitrate, "1000K");
}

#[test]
fn test_cli_required_arguments() {
    let app = build_cli();
    
    // Test that URL is required unless --activate or --license is present
    let result = app.clone().try_get_matches_from(vec!["rustloader"]);
    assert!(result.is_err());
    
    // Test that --activate doesn't require a URL
    let result = app.clone().try_get_matches_from(vec!["rustloader", "--activate", "LICENSE-KEY"]);
    assert!(result.is_ok());
    
    // Test that --license doesn't require a URL
    let result = app.clone().try_get_matches_from(vec!["rustloader", "--license"]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_invalid_arguments() {
    let app = build_cli();
    
    // Test invalid quality option
    let result = app.clone().try_get_matches_from(vec![
        "rustloader", 
        "https://example.com", 
        "--quality", 
        "invalid"
    ]);
    assert!(result.is_err());
    
    // Test invalid format option
    let result = app.clone().try_get_matches_from(vec![
        "rustloader", 
        "https://example.com", 
        "--format", 
        "invalid"
    ]);
    assert!(result.is_err());
}