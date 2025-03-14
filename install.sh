#!/bin/bash

# Rustloader Installation Script
# This script installs rustloader and all its dependencies

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}    Rustloader Installation Script      ${NC}"
echo -e "${BLUE}========================================${NC}"

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="Linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macOS"
elif [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    OS="Windows"
else
    OS="Unknown"
fi

echo -e "${GREEN}Detected operating system: ${OS}${NC}"

# Check if running with sudo/root on Linux
if [[ "$OS" == "Linux" ]] && [[ $EUID -ne 0 ]]; then
    echo -e "${YELLOW}Warning: This script may need sudo privileges to install dependencies.${NC}"
    echo -e "${YELLOW}If it fails, please run it again with sudo.${NC}"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${RED}Installation cancelled.${NC}"
        exit 1
    fi
fi

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to handle interrupted installations
cleanup_interrupted_install() {
    echo -e "${YELLOW}Cleaning up interrupted installation...${NC}"
    
    # Remove partial downloads
    if [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
        echo -e "${GREEN}Removed temporary files${NC}"
    fi
    
    # Check for and remove partial binary installation
    if [ -f "$HOME/.cargo/bin/rustloader.partial" ]; then
        rm -f "$HOME/.cargo/bin/rustloader.partial"
        echo -e "${GREEN}Removed partial binary installation${NC}"
    fi
    
    echo -e "${GREEN}Cleanup complete. Please restart the installation.${NC}"
    exit 1
}

# Add trap to handle interruptions
trap cleanup_interrupted_install INT TERM

# Install Rust if not installed
install_rust() {
    if ! command_exists cargo; then
        echo -e "${YELLOW}Rust is not installed. Installing Rust...${NC}"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        echo -e "${GREEN}Rust installed successfully.${NC}"
    else
        echo -e "${GREEN}Rust is already installed.${NC}"
    fi
}

# Install dependencies based on OS
install_dependencies() {
    echo -e "${BLUE}Installing dependencies...${NC}"
    
    case $OS in
        "Linux")
            # Show more detailed guidance based on distribution
            if command_exists apt; then
                echo -e "${YELLOW}Detected Debian/Ubuntu-based system${NC}"
                echo -e "${YELLOW}Installing dependencies using apt...${NC}"
                sudo apt update
                sudo apt install -y python3 python3-pip ffmpeg libssl-dev pkg-config
                echo -e "${GREEN}Dependencies installed. If you encounter any issues, consider running:${NC}"
                echo -e "${GREEN}sudo apt install python3-venv build-essential${NC}"
            elif command_exists dnf; then
                echo -e "${YELLOW}Detected Fedora/RHEL-based system${NC}"
                echo -e "${YELLOW}Installing dependencies using dnf...${NC}"
                sudo dnf install -y python3 python3-pip ffmpeg openssl-devel pkgconfig
                echo -e "${GREEN}Dependencies installed. If you encounter any issues, consider running:${NC}"
                echo -e "${GREEN}sudo dnf groupinstall 'Development Tools'${NC}"
            elif command_exists pacman; then
                echo -e "${YELLOW}Detected Arch-based system${NC}"
                echo -e "${YELLOW}Installing dependencies using pacman...${NC}"
                sudo pacman -Sy python python-pip ffmpeg openssl pkg-config
                echo -e "${GREEN}Dependencies installed. If you encounter any issues, consider running:${NC}"
                echo -e "${GREEN}sudo pacman -S base-devel${NC}"
            else
                echo -e "${RED}Unsupported Linux distribution.${NC}"
                echo -e "${YELLOW}Please install these packages manually:${NC}"
                echo -e "${YELLOW}- python3 and pip (package manager varies by distribution)${NC}"
                echo -e "${YELLOW}- ffmpeg (package manager varies by distribution)${NC}"
                echo -e "${YELLOW}- openssl-dev or libssl-dev (package manager varies by distribution)${NC}"
                echo -e "${YELLOW}- pkg-config (package manager varies by distribution)${NC}"
                echo -e "${YELLOW}After installing dependencies, run:${NC}"
                echo -e "${YELLOW}pip3 install --user --upgrade yt-dlp${NC}"
                exit 1
            fi
            
            # Provide guidance for common Linux issues
            echo -e "${BLUE}===============================================${NC}"
            echo -e "${BLUE}Linux-specific troubleshooting tips:${NC}"
            echo -e "${YELLOW}1. If 'command not found' errors occur after installation:${NC}"
            echo -e "   Run: ${GREEN}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
            echo -e "${YELLOW}2. If permission errors occur:${NC}"
            echo -e "   Run: ${GREEN}chmod +x \$HOME/.local/bin/yt-dlp${NC}"
            echo -e "${YELLOW}3. For SSL certificate errors:${NC}"
            echo -e "   Run: ${GREEN}pip install --upgrade certifi${NC}"
            echo -e "${BLUE}===============================================${NC}"
            
            # Install yt-dlp
            pip3 install --user --upgrade yt-dlp
            ;;
            
        "macOS")
            if ! command_exists brew; then
                echo -e "${YELLOW}Homebrew not found. Installing Homebrew...${NC}"
                /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
            fi
            
            echo -e "${YELLOW}Installing dependencies with Homebrew...${NC}"
            brew install python ffmpeg yt-dlp openssl@3 pkg-config
            
            # Set up environment for OpenSSL
            export OPENSSL_DIR=$(brew --prefix openssl@3)
            echo "export OPENSSL_DIR=$(brew --prefix openssl@3)" >> ~/.bash_profile
            echo "export OPENSSL_DIR=$(brew --prefix openssl@3)" >> ~/.zshrc
            
            # macOS-specific troubleshooting tips
            echo -e "${BLUE}===============================================${NC}"
            echo -e "${BLUE}macOS-specific troubleshooting tips:${NC}"
            echo -e "${YELLOW}1. If you encounter permission issues:${NC}"
            echo -e "   Run: ${GREEN}sudo chown -R $(whoami) /usr/local/share/zsh /usr/local/share/zsh/site-functions${NC}"
            echo -e "${YELLOW}2. If brew commands fail:${NC}"
            echo -e "   Run: ${GREEN}brew doctor${NC}"
            echo -e "${YELLOW}3. If OpenSSL is not found during build:${NC}"
            echo -e "   Run: ${GREEN}export OPENSSL_DIR=\$(brew --prefix openssl@3)${NC}"
            echo -e "${BLUE}===============================================${NC}"
            ;;
            
        "Windows")
            echo -e "${YELLOW}Windows detected. This script has limited support for Windows.${NC}"
            
            if ! command_exists choco; then
                echo -e "${RED}Chocolatey not found. Please install it first: https://chocolatey.org/install${NC}"
                echo -e "${YELLOW}Or install Python, ffmpeg, and yt-dlp manually.${NC}"
                
                # Windows-specific manual installation instructions
                echo -e "${BLUE}===============================================${NC}"
                echo -e "${BLUE}Windows manual installation steps:${NC}"
                echo -e "${YELLOW}1. Install Python from: https://www.python.org/downloads/${NC}"
                echo -e "${YELLOW}2. Install ffmpeg from: https://www.gyan.dev/ffmpeg/builds/${NC}"
                echo -e "${YELLOW}3. Install yt-dlp with: pip install --user --upgrade yt-dlp${NC}"
                echo -e "${YELLOW}4. Add Python and ffmpeg to your PATH${NC}"
                echo -e "${BLUE}===============================================${NC}"
                exit 1
            fi
            
            echo -e "${YELLOW}Installing dependencies with Chocolatey...${NC}"
            choco install -y python ffmpeg openssl
            
            # Install yt-dlp with pip
            echo -e "${YELLOW}Installing yt-dlp...${NC}"
            pip install --user --upgrade yt-dlp
            
            # Windows-specific troubleshooting tips
            echo -e "${BLUE}===============================================${NC}"
            echo -e "${BLUE}Windows-specific troubleshooting tips:${NC}"
            echo -e "${YELLOW}1. If 'command not found' errors occur:${NC}"
            echo -e "   Ensure Python Scripts folder is in your PATH:"
            echo -e "   ${GREEN}%UserProfile%\\AppData\\Local\\Programs\\Python\\Python3X\\Scripts${NC}"
            echo -e "${YELLOW}2. If ffmpeg is not found:${NC}"
            echo -e "   Verify ffmpeg is in your PATH or manually install to:"
            echo -e "   ${GREEN}C:\\ffmpeg\\bin${NC}"
            echo -e "${YELLOW}3. To restart your PATH without restarting:${NC}"
            echo -e "   Open a new PowerShell/CMD window"
            echo -e "${BLUE}===============================================${NC}"
            ;;
            
        *)
            echo -e "${RED}Unsupported operating system: $OS${NC}"
            echo -e "${YELLOW}Please install Python, ffmpeg, and yt-dlp manually.${NC}"
            exit 1
            ;;
    esac
    
    echo -e "${GREEN}Dependencies installed successfully.${NC}"
}

# Verify dependencies
verify_dependencies() {
    echo -e "${BLUE}Verifying dependencies...${NC}"
    
    local missing=0
    
    if ! command_exists yt-dlp; then
        echo -e "${RED}yt-dlp not found in PATH${NC}"
        missing=1
    else
        echo -e "${GREEN}yt-dlp is installed: $(yt-dlp --version 2>&1 | head -n 1)${NC}"
    fi
    
    if ! command_exists ffmpeg; then
        echo -e "${RED}ffmpeg not found in PATH${NC}"
        missing=1
    else
        echo -e "${GREEN}ffmpeg is installed: $(ffmpeg -version 2>&1 | head -n 1)${NC}"
    fi
    
    if [[ $missing -eq 1 ]]; then
        echo -e "${YELLOW}Some dependencies are missing. Please install them manually and run this script again.${NC}"
        return 1
    fi
    
    return 0
}

# Install rustloader
install_rustloader() {
    echo -e "${BLUE}Installing rustloader...${NC}"
    
    # Create a temporary directory
    TMP_DIR=$(mktemp -d)
    echo -e "${YELLOW}Created temporary directory: $TMP_DIR${NC}"
    
    # Clone the repository
    echo -e "${YELLOW}Cloning rustloader repository...${NC}"
    git clone https://github.com/ibra2000sd/rustloader.git "$TMP_DIR/rustloader"
    
    # Build and install
    echo -e "${YELLOW}Building and installing rustloader...${NC}"
    cd "$TMP_DIR/rustloader"
    
    # First build with partial extension to avoid interruptions leaving broken install
    cargo build --release --target-dir "$TMP_DIR/rustloader/target"
    
    # Check if build was successful
    if [ -f "$TMP_DIR/rustloader/target/release/rustloader" ]; then
        # Copy to cargo bin directory
        cp "$TMP_DIR/rustloader/target/release/rustloader" "$HOME/.cargo/bin/rustloader"
        echo -e "${GREEN}Rustloader binary installed successfully.${NC}"
    else
        echo -e "${RED}Build failed. Please check error messages above.${NC}"
        cleanup_interrupted_install
    fi
    
    # Clean up
    echo -e "${YELLOW}Cleaning up...${NC}"
    cd - > /dev/null
    rm -rf "$TMP_DIR"
    
    echo -e "${GREEN}Rustloader installed successfully.${NC}"
}

# Add rustloader to PATH if not already there
setup_path() {
    echo -e "${BLUE}Setting up PATH...${NC}"
    
    # Check if ~/.cargo/bin is in PATH
    if [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
        echo -e "${YELLOW}Adding ~/.cargo/bin to PATH...${NC}"
        
        # Detect shell
        SHELL_NAME=$(basename "$SHELL")
        
        case $SHELL_NAME in
            "bash")
                PROFILE_FILE="$HOME/.bashrc"
                ;;
            "zsh")
                PROFILE_FILE="$HOME/.zshrc"
                ;;
            *)
                PROFILE_FILE="$HOME/.profile"
                ;;
        esac
        
        echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$PROFILE_FILE"
        echo -e "${GREEN}Added ~/.cargo/bin to PATH in $PROFILE_FILE${NC}"
        echo -e "${YELLOW}Please run 'source $PROFILE_FILE' or restart your terminal to apply changes.${NC}"
    else
        echo -e "${GREEN}~/.cargo/bin is already in PATH.${NC}"
    fi
}

# Test installation
test_installation() {
    echo -e "${BLUE}Testing rustloader installation...${NC}"
    
    if command_exists rustloader; then
        echo -e "${GREEN}Rustloader is installed and in PATH.${NC}"
        echo -e "${YELLOW}Rustloader version:${NC}"
        rustloader --version
        echo -e "${GREEN}Installation successful!${NC}"
    else
        echo -e "${RED}Rustloader is not in PATH. Please restart your terminal or add ~/.cargo/bin to your PATH manually.${NC}"
        echo -e "${RED}Installation may have failed.${NC}"
        exit 1
    fi
}

# Clean up existing download counter if it exists
cleanup_existing_data() {
    echo -e "${BLUE}Checking for existing data...${NC}"
    
    # Define paths based on OS
    local data_dir=""
    if [[ "$OS" == "Linux" ]]; then
        data_dir="$HOME/.local/share/rustloader"
    elif [[ "$OS" == "macOS" ]]; then
        data_dir="$HOME/Library/Application Support/rustloader"
    elif [[ "$OS" == "Windows" ]]; then
        data_dir="$APPDATA/rustloader"
    fi
    
    if [[ -d "$data_dir" ]]; then
        echo -e "${YELLOW}Found existing rustloader data directory: $data_dir${NC}"
        read -p "Would you like to clean up old data files? (Recommended for updates) (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo -e "${YELLOW}Removing old data files...${NC}"
            rm -f "$data_dir/download_counter.dat"
            echo -e "${GREEN}Old data files removed.${NC}"
        fi
    fi
}

# Function for verifying full installation
function verify_full_installation() {
    echo -e "${BLUE}Verifying installation...${NC}"
    
    local errors=0
    
    # Check if rustloader is in PATH
    if ! command_exists rustloader; then
        echo -e "${RED}ERROR: rustloader not found in PATH${NC}"
        echo -e "${YELLOW}Try manually adding it with: export PATH=\"\$HOME/.cargo/bin:\$PATH\"${NC}"
        errors=$((errors+1))
    fi
    
    # Check if dependencies are accessible
    if ! command_exists yt-dlp; then
        echo -e "${RED}ERROR: yt-dlp not found in PATH${NC}"
        echo -e "${YELLOW}Try installing manually with: pip install --user --upgrade yt-dlp${NC}"
        errors=$((errors+1))
    fi
    
    if ! command_exists ffmpeg; then
        echo -e "${RED}ERROR: ffmpeg not found in PATH${NC}"
        echo -e "${YELLOW}Please install ffmpeg using your package manager${NC}"
        errors=$((errors+1))
    fi
    
    if [ $errors -eq 0 ]; then
        echo -e "${GREEN}All components verified successfully!${NC}"
    else
        echo -e "${RED}Found $errors issue(s) with installation.${NC}"
        echo -e "${YELLOW}Please resolve these issues or run the installation script again.${NC}"
    fi
}

# Display final instructions
display_instructions() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${GREEN}Rustloader has been successfully installed!${NC}"
    echo -e "${GREEN}You can now use it by running 'rustloader' in your terminal.${NC}"
    echo -e "${YELLOW}Basic Usage:${NC}"
    echo -e "  rustloader [URL] [OPTIONS]"
    echo -e "${YELLOW}Examples:${NC}"
    echo -e "  rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ                  # Download video"
    echo -e "  rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --format mp3     # Download audio"
    echo -e "  rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --quality 720    # Set quality"
    echo -e "  rustloader --help                                                        # Show all options"
    echo -e "${YELLOW}Pro Version:${NC}"
    echo -e "  rustloader --activate YOUR_LICENSE_KEY                                   # Activate Pro"
    echo -e "  rustloader --license                                                     # Show license info"
    
    # OS-specific post-installation notes
    case $OS in
        "Linux")
            echo -e "${YELLOW}Linux Notes:${NC}"
            echo -e "  • If rustloader is not found, run: source ~/.bashrc (or ~/.zshrc)"
            echo -e "  • For system-wide installation, consider running: sudo cp ~/.cargo/bin/rustloader /usr/local/bin/"
            ;;
        "macOS")
            echo -e "${YELLOW}macOS Notes:${NC}"
            echo -e "  • If you encounter permission issues with dependencies, run: brew doctor"
            echo -e "  • Make sure your PATH includes: $HOME/.cargo/bin"
            ;;
        "Windows")
            echo -e "${YELLOW}Windows Notes:${NC}"
            echo -e "  • You may need to restart your terminal or computer for PATH changes to take effect"
            echo -e "  • If rustloader is not found, check that %USERPROFILE%\\.cargo\\bin is in your PATH"
            ;;
    esac
    
    echo -e "${BLUE}========================================${NC}"
    
    # Inform about support channels
    echo -e "${GREEN}Need help or have questions?${NC}"
    echo -e "  • Visit: https://rustloader.com/docs"
    echo -e "  • Report issues: https://github.com/ibra2000sd/rustloader/issues"
    echo -e "${BLUE}========================================${NC}"
}

# Main installation process
main() {
    echo -e "${BLUE}Starting installation...${NC}"
    
    install_rust
    install_dependencies
    
    if ! verify_dependencies; then
        echo -e "${RED}Dependency verification failed. Exiting.${NC}"
        exit 1
    fi
    
    install_rustloader
    setup_path
    cleanup_existing_data
    test_installation
    verify_full_installation
    display_instructions
}

# Run the installation
main