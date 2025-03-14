# Rustloader

<div align="center">

**Advanced Video Downloader built with Rust**

[![GitHub license](https://img.shields.io/github/license/ibra2000sd/rustloader)](https://github.com/ibra2000sd/rustloader/blob/main/LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/ibra2000sd/rustloader)](https://github.com/ibra2000sd/rustloader/stargazers)

</div>

Rustloader is a powerful, versatile command-line tool for downloading videos and audio from various online platforms. Built with Rust for maximum performance, reliability, and security.

## Table of Contents
- [Features](#features)
- [Required Dependencies](#required-dependencies)
- [Installation](#installation)
  - [Method 1: Automatic Installation Script](#method-1-automatic-installation-script-recommended)
  - [Method 2: Install from Source](#method-2-install-from-source)
  - [Method 3: Manual Dependencies Installation](#method-3-manual-dependencies-installation)
  - [Adding to System PATH](#adding-to-system-path)
- [Usage](#usage)
  - [Basic Usage](#basic-usage)
  - [Getting Help](#getting-help)
  - [Examples](#examples)
  - [Available Options](#available-options)
  - [Pro Version Activation](#pro-version-activation)
- [Uninstallation](#uninstallation)
- [Troubleshooting](#troubleshooting)
- [Security Features](#security-features)
- [License](#license)
- [Acknowledgments](#acknowledgments)
- [Contributing](#contributing)
- [Support](#support)



## Features

### Free Version
- Download videos up to 720p quality
- Extract MP3 audio at 128kbps
- Download specific segments using start and end time markers
- Download entire playlists
- Automatically fetch subtitles
- Enhanced progress tracking with file size, speed, and ETA display
- Desktop notifications when downloads complete
- Automatic dependency checking and updates
- Advanced security protections
- Cross-platform compatibility with OS-specific optimizations

### Pro Version
- High quality video downloads (1080p, 4K, 8K)
- High-fidelity audio extraction (320kbps, FLAC)
- No daily download limits
- Multi-threaded downloads for maximum speed
- Priority updates and support

## Installation

## Installation

### Method 1: Automatic Installation Script (Recommended)

For Linux and macOS users, we provide an automatic installation script that handles everything for you:

```bash
curl -sSL https://raw.githubusercontent.com/ibra2000sd/rustloader/main/install.sh | bash
```

This script will:
- Install Rust if not already installed
- Install all dependencies (yt-dlp and ffmpeg)
- Build and install rustloader
- Add rustloader to your system PATH
- Verify the installation works correctly
- Provide platform-specific troubleshooting guidance

For security-conscious users, you can download the script first, review it, and then run it:

```bash
curl -O https://raw.githubusercontent.com/ibra2000sd/rustloader/main/install.sh
chmod +x install.sh
./install.sh
```

#### For enhanced security (recommended):
```bash
curl -sSL https://raw.githubusercontent.com/ibra2000sd/rustloader/main/secure_install.sh | bash
```


### Method 2: Install from Source

1. **Install Rust and Cargo** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone the repository**:
   ```bash
   git clone https://github.com/ibra2000sd/rustloader.git
   cd rustloader
   ```

3. **Build and install**:
   ```bash
   cargo install --path .
   ```

### Method 3: Manual Dependencies Installation

Rustloader will check for and notify you about missing dependencies, but you can install them ahead of time:

## Required Dependencies

Rustloader depends on these external tools:

- **yt-dlp** - For video extraction
- **ffmpeg** - For media processing

The automatic installation script will install these for you. If you're installing manually, see the Manual Dependencies Installation section below.

#### On macOS (using Homebrew):
```bash
brew install yt-dlp ffmpeg
```

#### On Linux (Debian/Ubuntu):
```bash
sudo apt update
sudo apt install python3 python3-pip ffmpeg libssl-dev pkg-config
pip3 install --user --upgrade yt-dlp
```

#### On Linux (Fedora/RHEL):
```bash
sudo dnf install python3 python3-pip ffmpeg openssl-devel pkgconfig
pip3 install --user --upgrade yt-dlp
```

#### On Linux (Arch):
```bash
sudo pacman -Sy python python-pip ffmpeg openssl pkg-config
pip3 install --user --upgrade yt-dlp
```

#### On Windows:
1. Install Python: https://www.python.org/downloads/
2. Install ffmpeg: https://ffmpeg.org/download.html#build-windows
3. Install yt-dlp: `pip install --user --upgrade yt-dlp`
4. Add Python and ffmpeg to your PATH

### Adding to System PATH

#### Linux/macOS

If you've installed using `cargo install`, the binary is automatically added to your PATH at `~/.cargo/bin/rustloader`.

To manually add to PATH:

1. **Find the binary location**:
   ```bash
   which rustloader
   ```

2. **Add to your shell profile** (`.bashrc`, `.zshrc`, etc.):
   ```bash
   echo 'export PATH=$PATH:/path/to/rustloader/binary' >> ~/.bashrc
   source ~/.bashrc
   ```

#### Windows

1. **Find the binary location** (typically in `%USERPROFILE%\.cargo\bin`)

2. **Add to PATH**:
   - Right-click on 'This PC' or 'My Computer' and select 'Properties'
   - Click on 'Advanced system settings'
   - Click the 'Environment Variables' button
   - Under 'System variables', find and select 'Path', then click 'Edit'
   - Click 'New' and add the path to the directory containing rustloader.exe
   - Click 'OK' on all dialogs to save changes

## Usage

### Basic Usage

```bash
rustloader [URL] [OPTIONS]
```

### Getting Help

To see all available options and commands:

```bash
rustloader --help
# or
rustloader -h
```

This displays a comprehensive help message with all available options, arguments, and their descriptions.

### Examples

1. **Download a video in default quality**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ
   ```

2. **Download in specific quality**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --quality 720
   ```

3. **Download audio only**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --format mp3
   ```

4. **Download a specific section**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --start-time 00:01:30 --end-time 00:02:45
   ```

5. **Download with subtitles**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --subs
   ```

6. **Download a playlist**:
   ```bash
   rustloader https://www.youtube.com/playlist?list=PLxxxxxxx --playlist
   ```

7. **Specify output directory**:
   ```bash
   rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --output-dir ~/Videos/music
   ```

### Pro Version Activation

If you have purchased a Pro license, you can activate it with:

```bash
rustloader --activate YOUR_LICENSE_KEY
```

To check your license status:

```bash
rustloader --license
```

### Available Options

| Option | Short | Description |
|--------|-------|-------------|
| `--help` | `-h` | Display help information |
| `--version` | `-V` | Display version information |
| `--quality` | `-q` | Video quality (480, 720, 1080) |
| `--format` | `-f` | Output format (mp4, mp3) |
| `--start-time` | `-s` | Start time (HH:MM:SS) |
| `--end-time` | `-e` | End time (HH:MM:SS) |
| `--playlist` | `-p` | Download entire playlist |
| `--subs` | | Download subtitles if available |
| `--output-dir` | `-o` | Specify custom output directory |
| `--bitrate` | | Set video bitrate (e.g., 1000K) |
| `--activate` | | Activate a Pro license |
| `--license` | | Display license information |

### Examples

#### Download a video in specific quality:
```bash
rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --quality 720
```

#### Download audio only:
```bash
rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --format mp3
```

#### Download a specific section:
```bash
rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --start-time 00:01:30 --end-time 00:02:45
```

#### Download with subtitles:
```bash
rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --subs
```

#### Download a playlist:
```bash
rustloader https://www.youtube.com/playlist?list=PLxxxxxxx --playlist
```

#### Specify output directory:
```bash
rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --output-dir ~/Videos/music
```

## Uninstallation

To remove Rustloader while keeping dependencies intact:

```bash
curl -sSL https://raw.githubusercontent.com/ibra2000sd/rustloader/main/uninstall.sh | bash
```

For security-conscious users, you can download, review, and then run the script:

```bash
curl -O https://raw.githubusercontent.com/ibra2000sd/rustloader/main/uninstall.sh
chmod +x uninstall.sh
./uninstall.sh
```

The uninstallation script will:
- Remove the Rustloader binary
- Clean up configuration files (optional)
- Remove data files (optional)
- Remove downloaded content (optional)
- Keep yt-dlp and ffmpeg intact for other applications

## Troubleshooting

### Daily Download Limit

Free version users are limited to 5 downloads per day. To remove this limitation, consider upgrading to the Pro version.

### 403 Forbidden Errors

If you encounter a 403 Forbidden error, it might be because YouTube is detecting automated downloads.

Solutions:
1. Update yt-dlp to the latest version (Rustloader attempts this automatically)
2. Create a cookies.txt file in your home directory (~/.cookies.txt) by exporting cookies from your browser

### Installation Issues

#### ffmpeg not found:
Make sure ffmpeg is installed and in your PATH. You can verify with:
```bash
ffmpeg -version
```

#### yt-dlp not found:
Verify yt-dlp is installed correctly:
```bash
yt-dlp --version
```

If needed, reinstall:
```bash
pip install --user --upgrade yt-dlp
```

### Network Issues

If you experience connection timeouts or interruptions:
- Check your internet connection
- Ensure your firewall isn't blocking Rustloader
- Try using a different network if possible

## Security Features

Rustloader includes several security features:
- Secure license verification system with tamper detection
- Path validation to prevent directory traversal attacks
- Strict input sanitization for all command arguments
- Safe file operations with proper permissions checking
- Anti-tampering protections for download counters
- Enhanced URL validation to prevent command injection
- Secure update verification with signature checking

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) for the video extraction capabilities
- [ffmpeg](https://ffmpeg.org/) for media processing

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Support

If you encounter any issues or have questions:
- Check the troubleshooting section in this README
- Open an issue on GitHub