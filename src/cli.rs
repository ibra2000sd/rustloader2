// src/cli.rs

use clap::{Arg, ArgAction, Command};

/// Build the command-line interface for the application
pub fn build_cli() -> Command {
    let mut app = Command::new("rustloader")
        .version("1.0.0")
        .author("Ibrahim Mohamed")
        .about("Advanced video downloader for various content sources")
        .subcommand_negates_reqs(true)
        .subcommand(
            Command::new("download")
                .about("Download a video or audio")
                .arg(
                    Arg::new("url")
                        .help("The URL of the video or playlist to download")
                        .required(true)
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
                .arg(
                    Arg::new("priority")
                        .long("priority")
                        .help("Set download priority (low, normal, high, critical)")
                        .value_parser(["low", "normal", "high", "critical"])
                        .default_value("normal"),
                )
                .arg(
                    Arg::new("add-to-queue")
                        .long("queue")
                        .help("Add to download queue instead of downloading immediately")
                        .action(ArgAction::SetTrue),
                )
        )
        .subcommand(
            Command::new("queue")
                .about("Manage download queue")
                .subcommand(Command::new("list").about("List all downloads in the queue"))
                .subcommand(Command::new("pause-all").about("Pause all active downloads"))
                .subcommand(Command::new("resume-all").about("Resume all paused downloads"))
                .subcommand(
                    Command::new("pause")
                        .about("Pause a specific download")
                        .arg(
                            Arg::new("id")
                                .help("Download ID to pause")
                                .required(true)
                                .index(1),
                        ),
                )
                .subcommand(
                    Command::new("resume")
                        .about("Resume a specific download")
                        .arg(
                            Arg::new("id")
                                .help("Download ID to resume")
                                .required(true)
                                .index(1),
                        ),
                )
                .subcommand(
                    Command::new("cancel")
                        .about("Cancel a specific download")
                        .arg(
                            Arg::new("id")
                                .help("Download ID to cancel")
                                .required(true)
                                .index(1),
                        ),
                )
                .subcommand(
                    Command::new("priority")
                        .about("Change a download's priority")
                        .arg(
                            Arg::new("id")
                                .help("Download ID")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("level")
                                .help("Priority level (low, normal, high, critical)")
                                .required(true)
                                .index(2)
                                .value_parser(["low", "normal", "high", "critical"]),
                        ),
                )
                .subcommand(Command::new("clear-completed").about("Remove completed downloads from the queue"))
                .subcommand(Command::new("clear-failed").about("Clear failed downloads from the queue")),
        )
        // Support for just URL as before for backward compatibility
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
