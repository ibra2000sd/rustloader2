// src/cli.rs

use clap::{Arg, ArgAction, Command};

/// Build the command-line interface for the application
pub fn build_cli() -> Command {
    let mut app = Command::new("rustloader")
        .version("1.0.0")
        .author("Ibrahim Mohamed")
        .about("Advanced video downloader for various content sources")
        .arg(
            Arg::new("url")
                .help("The URL of the video or playlist to download")
                .required_unless_present_any(["activate-license", "license-info"])
                .index(1),
        )
        .arg(
            Arg::new("quality")
                .long("quality")
                .short('q')
                .help("Specify the desired quality (480, 720, 1080)")
                .value_parser(["480", "720", "1080"]),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .help("Specify the format (mp4 or mp3)")
                .value_parser(["mp4", "mp3"]),
        )
        .arg(
            Arg::new("start-time")
                .long("start-time")
                .short('s')
                .help("Specify the start time of the clip (e.g., 00:01:00)")
                .value_name("START_TIME"),
        )
        .arg(
            Arg::new("end-time")
                .long("end-time")
                .short('e')
                .help("Specify the end time of the clip (e.g., 00:02:00)")
                .value_name("END_TIME"),
        )
        .arg(
            Arg::new("playlist")
                .long("playlist")
                .short('p')
                .help("Download entire playlist")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("subtitles")
                .long("subs")
                .help("Download subtitles if available")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output-dir")
                .long("output-dir")
                .short('o')
                .help("Specify custom output directory")
                .value_name("DIRECTORY"),
        )
        .arg(
            Arg::new("video-bitrate")
                .long("bitrate")
                .help("Set video bitrate (e.g., 1000K)")
                .value_name("BITRATE"),
        )
        // Add license activation argument
        .arg(
            Arg::new("activate-license")
                .long("activate")
                .help("Activate a Pro license using the provided key")
                .value_name("LICENSE_KEY"),
        )
        // Add license info display argument
        .arg(
            Arg::new("license-info")
                .long("license")
                .help("Display current license information")
                .action(ArgAction::SetTrue),
        );

    // Only include the force flag in debug builds
    #[cfg(debug_assertions)]
    {
        app = app.arg(
            Arg::new("force")
                .long("force")
                .help("Force download, ignoring daily limits (development use only)")
                .action(ArgAction::SetTrue),
        );
    }

    app
}
