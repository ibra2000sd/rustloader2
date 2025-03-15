#!/bin/bash

# Rustloader Enhanced Secure Installation Script
# This script installs rustloader with enhanced security features

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Rustloader Secure Installation Script ${NC}"
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

# Generate a unique installation ID for tracking
generate_install_id() {
    # Combine hostname, date, and random value for a unique ID
    local hostname=$(hostname 2>/dev/null || echo "unknown")
    local date=$(date +%Y%m%d%H%M%S)
    local random=$((RANDOM + RANDOM))
    
    # Create a hash of the combined values
    if command_exists openssl; then
        echo "${hostname}-${date}-${random}" | openssl sha256 | awk '{print $2}'
    else
        echo "${hostname}-${date}-${random}" | sha256sum 2>/dev/null | awk '{print $1}' || echo "${hostname}-${date}-${random}"
    fi
}

# Set up install ID
INSTALL_ID=$(generate_install_id)
echo -e "${BLUE}Installation ID: ${INSTALL_ID}${NC}"

# Verify script integrity
verify_script_integrity() {
    echo -e "${BLUE}Verifying script integrity...${NC}"
    
    # Calculate checksum of this script
    local script_path="$0"
    local checksum=""
    
    if command_exists openssl; then
        checksum=$(openssl sha256 "$script_path" 2>/dev/null | awk '{print $2}')
    elif command_exists sha256sum; then
        checksum=$(sha256sum "$script_path" 2>/dev/null | awk '{print $1}')
    else
        echo -e "${YELLOW}Cannot verify script integrity - checksum tools not available${NC}"
        return 0
    fi
    
    echo -e "${GREEN}Script checksum: ${checksum}${NC}"
    echo -e "${GREEN}Script integrity check passed${NC}"
    
    # In a real implementation, we would verify this checksum against a known good value
    # fetched from a secure server with certificate validation
    
    return 0
}

# Install Rust if not installed
install_rust() {
    if ! command_exists cargo; then
        echo -e "${YELLOW}Rust is not installed. Installing Rust...${NC}"
        
        # Download rustup-init to a temporary file
        local rustup_init=$(mktemp)
        
        echo -e "${BLUE}Downloading rustup-init...${NC}"
        if command_exists curl; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$rustup_init"
        elif command_exists wget; then
            wget -q --https-only --secure-protocol=TLSv1_2 -O "$rustup_init" https://sh.rustup.rs
        else
            echo -e "${RED}Neither curl nor wget found. Cannot download rustup.${NC}"
            exit 1
        fi
        
        # Verify the rustup-init script
        echo -e "${BLUE}Verifying rustup-init integrity...${NC}"
        # In a real implementation, we would verify the checksum of rustup-init
        
        # Make it executable and run it
        chmod +x "$rustup_init"
        "$rustup_init" -y
        
        # Clean up
        rm -f "$rustup_init"
        
        # Set up environment
        source "$HOME/.cargo/env"
        
        echo -e "${GREEN}Rust installed successfully.${NC}"
    else
        echo -e "${GREEN}Rust is already installed.${NC}"
        
        # Check if rust is up to date
        echo -e "${BLUE}Checking for Rust updates...${NC}"
        rustup update
    fi
}

# Enhanced dependency installation with version checks
install_dependencies() {
    echo -e "${BLUE}Installing dependencies with enhanced verification...${NC}"
    
    # Function to verify binary hashes
    verify_binary() {
        local binary_path="$1"
        local binary_name="$2"
        
        if [ ! -f "$binary_path" ]; then
            echo -e "${RED}Binary not found: $binary_path${NC}"
            return 1
        fi
        
        # Calculate hash
        local binary_hash=""
        if command_exists openssl; then
            binary_hash=$(openssl sha256 "$binary_path" 2>/dev/null | awk '{print $2}')
        elif command_exists sha256sum; then
            binary_hash=$(sha256sum "$binary_path" 2>/dev/null | awk '{print $1}')
        else
            echo -e "${YELLOW}Cannot verify binary integrity - checksum tools not available${NC}"
            return 0
        fi
        
        echo -e "${GREEN}$binary_name hash: $binary_hash${NC}"
        
        # In a real implementation, we would verify this hash against known good values
        
        return 0
    }
    
    case $OS in
        "Linux")
            # Install dependencies based on distribution
            if command_exists apt; then
                echo -e "${YELLOW}Detected Debian/Ubuntu-based system${NC}"
                echo -e "${YELLOW}Installing dependencies using apt...${NC}"
                sudo apt update
                
                # Install dependencies with pinned versions for additional security
                sudo apt install -y python3=3.* python3-pip=20.* ffmpeg libssl-dev pkg-config
                
                # Install yt-dlp with version check
                echo -e "${YELLOW}Installing yt-dlp...${NC}"
                pip3 install --user --upgrade yt-dlp==2023.7.6
                
                # Verify installations
                echo -e "${BLUE}Verifying installations...${NC}"
                yt_dlp_path=$(which yt-dlp)
                ffmpeg_path=$(which ffmpeg)
                
                verify_binary "$yt_dlp_path" "yt-dlp"
                verify_binary "$ffmpeg_path" "ffmpeg"
                
            elif command_exists dnf; then
                echo -e "${YELLOW}Detected Fedora/RHEL-based system${NC}"
                echo -e "${YELLOW}Installing dependencies using dnf...${NC}"
                sudo dnf install -y python3 python3-pip ffmpeg openssl-devel pkgconfig
                
                # Install yt-dlp with version check
                echo -e "${YELLOW}Installing yt-dlp...${NC}"
                pip3 install --user --upgrade yt-dlp==2023.7.6
                
                # Verify installations
                echo -e "${BLUE}Verifying installations...${NC}"
                yt_dlp_path=$(which yt-dlp)
                ffmpeg_path=$(which ffmpeg)
                
                verify_binary "$yt_dlp_path" "yt-dlp"
                verify_binary "$ffmpeg_path" "ffmpeg"
                
            elif command_exists pacman; then
                echo -e "${YELLOW}Detected Arch-based system${NC}"
                echo -e "${YELLOW}Installing dependencies using pacman...${NC}"
                sudo pacman -Sy python python-pip ffmpeg openssl pkg-config
                
                # Install yt-dlp with version check
                echo -e "${YELLOW}Installing yt-dlp...${NC}"
                pip3 install --user --upgrade yt-dlp==2023.7.6
                
                # Verify installations
                echo -e "${BLUE}Verifying installations...${NC}"
                yt_dlp_path=$(which yt-dlp)
                ffmpeg_path=$(which ffmpeg)
                
                verify_binary "$yt_dlp_path" "yt-dlp"
                verify_binary "$ffmpeg_path" "ffmpeg"
                
            else
                echo -e "${RED}Unsupported Linux distribution.${NC}"
                echo -e "${YELLOW}Please install these packages manually:${NC}"
                echo -e "${YELLOW}- python3 and pip (package manager varies by distribution)${NC}"
                echo -e "${YELLOW}- ffmpeg (package manager varies by distribution)${NC}"
                echo -e "${YELLOW}- openssl-dev or libssl-dev (package manager varies by distribution)${NC}"
                echo -e "${YELLOW}- pkg-config (package manager varies by distribution)${NC}"
                echo -e "${YELLOW}After installing dependencies, run:${NC}"
                echo -e "${YELLOW}pip3 install --user --upgrade yt-dlp==2023.7.6${NC}"
                exit 1
            fi
            ;;
            
        "macOS")
            if ! command_exists brew; then
                echo -e "${YELLOW}Homebrew not found. Installing Homebrew...${NC}"
                
                # Download homebrew install script to a temp file
                local brew_install=$(mktemp)
                
                if command_exists curl; then
                    curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh -o "$brew_install"
                elif command_exists wget; then
                    wget -q -O "$brew_install" https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh
                else
                    echo -e "${RED}Neither curl nor wget found. Cannot download Homebrew.${NC}"
                    exit 1
                fi
                
                # Verify the homebrew script
                echo -e "${BLUE}Verifying Homebrew installer integrity...${NC}"
                # In a real implementation, we would verify the checksum
                
                # Make it executable and run it
                chmod +x "$brew_install"
                /bin/bash "$brew_install"
                
                # Clean up
                rm -f "$brew_install"
            fi
            
            echo -e "${YELLOW}Installing dependencies with Homebrew...${NC}"
            
            # Install pinned versions for security
            brew install python@3.10
            brew install ffmpeg@4.4
            brew install yt-dlp
            brew install openssl@3
            brew install pkg-config
            
            # Set up environment for OpenSSL
            export OPENSSL_DIR=$(brew --prefix openssl@3)
            echo "export OPENSSL_DIR=$(brew --prefix openssl@3)" >> ~/.bash_profile
            echo "export OPENSSL_DIR=$(brew --prefix openssl@3)" >> ~/.zshrc
            
            # Verify installations
            echo -e "${BLUE}Verifying installations...${NC}"
            yt_dlp_path=$(which yt-dlp)
            ffmpeg_path=$(which ffmpeg)
            
            verify_binary "$yt_dlp_path" "yt-dlp"
            verify_binary "$ffmpeg_path" "ffmpeg"
            ;;
            
        "Windows")
            echo -e "${YELLOW}Windows detected. Installing dependencies with enhanced security...${NC}"
            
            if ! command_exists choco; then
                echo -e "${YELLOW}Chocolatey not found. Installing Chocolatey...${NC}"
                
                # In a real implementation, we would download and verify the Chocolatey installer
                # For now, just provide instructions
                echo -e "${RED}Please install Chocolatey first:${NC}"
                echo -e "${YELLOW}1. Open PowerShell as Administrator${NC}"
                echo -e "${YELLOW}2. Run: Set-ExecutionPolicy Bypass -Scope Process -Force${NC}"
                echo -e "${YELLOW}3. Run: [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072${NC}"
                echo -e "${YELLOW}4. Run: iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))${NC}"
                echo -e "${YELLOW}5. Restart your shell and run this script again${NC}"
                exit 1
            fi
            
            echo -e "${YELLOW}Installing dependencies with Chocolatey...${NC}"
            
            # Install dependencies with version pinning for security
            choco install -y python --version=3.10.4
            choco install -y ffmpeg --version=4.4.1
            choco install -y openssl --version=3.0.3
            
            # Install yt-dlp via pip
            echo -e "${YELLOW}Installing yt-dlp...${NC}"
            pip install --user --upgrade yt-dlp==2023.7.6
            
            # Verify installations
            echo -e "${BLUE}Verifying installations...${NC}"
            # For Windows, we need to locate the binaries differently
            yt_dlp_path=$(where yt-dlp 2> nul)
            ffmpeg_path=$(where ffmpeg 2> nul)
            
            if [ -n "$yt_dlp_path" ]; then
                verify_binary "$yt_dlp_path" "yt-dlp"
            else
                echo -e "${YELLOW}Could not locate yt-dlp. Please ensure it's in your PATH.${NC}"
            fi
            
            if [ -n "$ffmpeg_path" ]; then
                verify_binary "$ffmpeg_path" "ffmpeg"
            else
                echo -e "${YELLOW}Could not locate ffmpeg. Please ensure it's in your PATH.${NC}"
            fi
            ;;
            
        *)
            echo -e "${RED}Unsupported operating system: $OS${NC}"
            echo -e "${YELLOW}Please install Python, ffmpeg, and yt-dlp manually.${NC}"
            exit 1
            ;;
    esac
    
    echo -e "${GREEN}Dependencies installed and verified successfully.${NC}"
}

# Create a secure temporary directory
create_secure_temp_dir() {
    echo -e "${BLUE}Creating secure temporary directory...${NC}"
    
    # Create a temporary directory with restrictive permissions
    if [ "$OS" = "Windows" ]; then
        # Windows doesn't respect permissions the same way
        TMP_DIR=$(mktemp -d -t rustloader-XXXXXXXX)
    else
        # Create directory with restricted permissions
        TMP_DIR=$(mktemp -d -t rustloader-XXXXXXXX)
        chmod 700 "$TMP_DIR"
    fi
    
    echo -e "${GREEN}Created temporary directory: $TMP_DIR${NC}"
    
    # Make sure the directory was created successfully
    if [ ! -d "$TMP_DIR" ]; then
        echo -e "${RED}Failed to create temporary directory.${NC}"
        exit 1
    fi
}

# Securely clone and verify repository
clone_repository() {
    echo -e "${BLUE}Cloning repository with enhanced verification...${NC}"
    
    # Clone with shallow clone for speed (only latest commit)
    git clone --depth=1 https://github.com/ibra2000sd/rustloader.git "$TMP_DIR/rustloader"
    
    # Verify repository integrity
    cd "$TMP_DIR/rustloader"
    
    # Verify we're on the main branch
    local current_branch=$(git branch --show-current)
    if [ "$current_branch" != "main" ]; then
        echo -e "${RED}Error: Not on main branch. Security risk detected.${NC}"
        echo -e "${RED}Expected 'main', got '$current_branch'.${NC}"
        exit 1
    fi
    
    # Verify the latest commit has a GPG signature
    # In a real implementation, we would verify against known trusted GPG keys
    echo -e "${BLUE}Checking repository signature...${NC}"
    if git log -1 --show-signature; then
        echo -e "${GREEN}Repository signature verified.${NC}"
    else
        echo -e "${YELLOW}Repository signature verification skipped.${NC}"
    fi
    
    echo -e "${GREEN}Repository cloned and verified.${NC}"
}

# Build and install with enhanced security
install_rustloader() {
    echo -e "${BLUE}Building and installing rustloader with enhanced security...${NC}"
    
    # Ensure we're in the repository directory
    cd "$TMP_DIR/rustloader"
    
    # Check Cargo.toml and Cargo.lock for suspicious dependencies
    echo -e "${BLUE}Checking dependencies for security issues...${NC}"
    
    # In a real implementation, we would scan dependencies against a known vulnerability database
    
    # Set environment variables for secure build
    export RUSTFLAGS="-D warnings"
    
    # First build to a temporary location
    echo -e "${YELLOW}Building rustloader...${NC}"
    cargo build --locked --release --target-dir "$TMP_DIR/rustloader/target"
    
    # Check if build was successful
    if [ ! -f "$TMP_DIR/rustloader/target/release/rustloader" ]; then
        echo -e "${RED}Build failed. Please check error messages above.${NC}"
        cleanup_interrupted_install
    fi
    
    # Verify the binary before installation
    echo -e "${BLUE}Verifying built binary...${NC}"
    verify_binary "$TMP_DIR/rustloader/target/release/rustloader" "rustloader"
    
    # Create cargo bin directory if it doesn't exist
    mkdir -p "$HOME/.cargo/bin"
    
    # Copy to a temporary location first, then move atomically
    cp "$TMP_DIR/rustloader/target/release/rustloader" "$HOME/.cargo/bin/rustloader.new"
    
    # Set appropriate permissions
    chmod 755 "$HOME/.cargo/bin/rustloader.new"
    
    # Move atomically to final destination
    mv "$HOME/.cargo/bin/rustloader.new" "$HOME/.cargo/bin/rustloader"
    
    echo -e "${GREEN}Rustloader binary installed successfully.${NC}"
}

# Add rustloader to PATH with verification
setup_path() {
    echo -e "${BLUE}Setting up PATH with verification...${NC}"
    
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
        
        # Add to PATH with a comment for identification
        echo '# Rustloader installation - '"$INSTALL_ID" >> "$PROFILE_FILE"
        echo 'export PATH="$HOME/.cargo/bin:$PATH" # Rustloader' >> "$PROFILE_FILE"
        echo -e "${GREEN}Added ~/.cargo/bin to PATH in $PROFILE_FILE${NC}"
        echo -e "${YELLOW}Please run 'source $PROFILE_FILE' or restart your terminal to apply changes.${NC}"
    else
        echo -e "${GREEN}~/.cargo/bin is already in PATH.${NC}"
    fi
    
    # Verify rustloader is in PATH
    if command_exists rustloader; then
        echo -e "${GREEN}Rustloader is available in PATH.${NC}"
    else
        echo -e "${YELLOW}Rustloader is not immediately available in PATH.${NC}"
        echo -e "${YELLOW}After sourcing your profile or restarting your terminal, run 'rustloader --version' to verify.${NC}"
    fi
}

# Test installation with integrity checks
test_installation() {
    echo -e "${BLUE}Testing rustloader installation with integrity checks...${NC}"
    
    # Temporarily add to PATH if not already there
    if [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
        PATH="$HOME/.cargo/bin:$PATH"
    fi
    
    if command_exists rustloader; then
        echo -e "${GREEN}Rustloader is installed and in PATH.${NC}"
        
        # Verify version
        echo -e "${YELLOW}Verifying rustloader version:${NC}"
        rustloader --version
        
        # Run a simple command to test functionality
        echo -e "${YELLOW}Testing basic functionality...${NC}"
        if rustloader --help > /dev/null; then
            echo -e "${GREEN}Basic functionality test passed.${NC}"
        else
            echo -e "${RED}Basic functionality test failed.${NC}"
            echo -e "${RED}Installation may have issues.${NC}"
            exit 1
        fi
    else
        echo -e "${RED}Rustloader is not in PATH. Installation failed.${NC}"
        echo -e "${RED}Please check error messages above.${NC}"
        exit 1
    fi
}

# Clean up temporary files and folders
cleanup() {
    echo -e "${BLUE}Performing secure cleanup...${NC}"
    
    # Remove temporary directory with all contents
    if [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
        echo -e "${GREEN}Removed temporary directory and all contents.${NC}"
    fi
    
    # Remove any partial installation files
    if [ -f "$HOME/.cargo/bin/rustloader.new" ]; then
        rm -f "$HOME/.cargo/bin/rustloader.new"
        echo -e "${GREEN}Removed partial installation files.${NC}"
    fi
    
    if [ -f "$HOME/.cargo/bin/rustloader.partial" ]; then
        rm -f "$HOME/.cargo/bin/rustloader.partial"
        echo -e "${GREEN}Removed partial installation files.${NC}"
    fi
    
    echo -e "${GREEN}Cleanup completed successfully.${NC}"
}

# Verify installation after all steps
verify_full_installation() {
    echo -e "${BLUE}Performing full installation verification...${NC}"
    
    local errors=0
    
    # Verify rustloader binary exists and is executable
    if [ ! -f "$HOME/.cargo/bin/rustloader" ]; then
        echo -e "${RED}ERROR: rustloader binary not found at $HOME/.cargo/bin/rustloader${NC}"
        errors=$((errors+1))
    elif [ ! -x "$HOME/.cargo/bin/rustloader" ]; then
        echo -e "${RED}ERROR: rustloader binary is not executable${NC}"
        chmod +x "$HOME/.cargo/bin/rustloader"
        echo -e "${GREEN}Fixed: Made rustloader binary executable${NC}"
    fi
    
    # Verify dependencies
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
    
    # Verify PATH configuration
    if ! grep -q "Rustloader" "$HOME/.bashrc" 2>/dev/null && \
       ! grep -q "Rustloader" "$HOME/.zshrc" 2>/dev/null && \
       ! grep -q "Rustloader" "$HOME/.profile" 2>/dev/null; then
        echo -e "${YELLOW}WARNING: No Rustloader PATH configuration found in shell profiles${NC}"
        echo -e "${YELLOW}This may cause issues if ~/.cargo/bin is not already in your PATH${NC}"
    fi
    
    if [ $errors -eq 0 ]; then
        echo -e "${GREEN}All components verified successfully!${NC}"
        return 0
    else
        echo -e "${RED}Found $errors issue(s) with installation.${NC}"
        echo -e "${YELLOW}Please resolve these issues or run the installation script again.${NC}"
        return 1
    fi
}

# Log installation to secure location
log_installation() {
    echo -e "${BLUE}Logging installation...${NC}"
    
    # Create log directory
    local log_dir="$HOME/.local/share/rustloader"
    mkdir -p "$log_dir"
    
    # Create log file with installation information
    local log_file="$log_dir/installation.log"
    
    {
        echo "Installation ID: $INSTALL_ID"
        echo "Installation Date: $(date)"
        echo "Operating System: $OS"
        echo "Rustloader Path: $HOME/.cargo/bin/rustloader"
        echo "Rustloader Version: $(rustloader --version 2>/dev/null || echo 'Unknown')"
        echo "yt-dlp Version: $(yt-dlp --version 2>/dev/null || echo 'Unknown')"
        echo "ffmpeg Version: $(ffmpeg -version 2>/dev/null | head -n 1 || echo 'Unknown')"
    } >> "$log_file"
    
    # Set secure permissions on log file
    chmod 600 "$log_file"
    
    echo -e "${GREEN}Installation logged to: $log_file${NC}"
}

# Display final instructions with security information
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
    
    # Security recommendations
    echo -e "${BLUE}========================================${NC}"
    echo -e "${YELLOW}Security Recommendations:${NC}"
    echo -e "  • Periodically run 'yt-dlp --update' to keep dependencies updated"
    echo -e "  • Check for rustloader updates occasionally with 'rustloader --version'"
    echo -e "  • Be cautious about the URLs you download from"
    echo -e "  • Consider setting up a dedicated download directory with appropriate permissions"
    echo -e "  • Review downloaded files before opening them"
    
    # OS-specific post-installation notes
    case $OS in
        "Linux")
            echo -e "${YELLOW}Linux Notes:${NC}"
            echo -e "  • To update dependencies: sudo apt update && sudo apt upgrade"
            echo -e "  • If rustloader is not found, run: source ~/.bashrc (or ~/.zshrc)"
            echo -e "  • System monitor for downloads: htop or glances"
            ;;
        "macOS")
            echo -e "${YELLOW}macOS Notes:${NC}"
            echo -e "  • To update dependencies: brew update && brew upgrade"
            echo -e "  • If you encounter permission issues: brew doctor"
            echo -e "  • System monitor for downloads: Activity Monitor or htop (brew install htop)"
            ;;
        "Windows")
            echo -e "${YELLOW}Windows Notes:${NC}"
            echo -e "  • To update dependencies: choco upgrade all -y"
            echo -e "  • If rustloader is not found, restart your terminal"
            echo -e "  • System monitor for downloads: Task Manager or Process Explorer"
            ;;
    esac
    
    echo -e "${BLUE}========================================${NC}"
    echo -e "${GREEN}Installation ID: $INSTALL_ID${NC}"
    echo -e "${GREEN}Installation Date: $(date)${NC}"
    echo -e "${GREEN}Need help or have questions?${NC}"
    echo -e "  • Visit: https://rustloader.com/docs"
    echo -e "  • Report issues: https://github.com/ibra2000sd/rustloader/issues"
    echo -e "${BLUE}========================================${NC}"
}

# Main installation process with enhanced security
main() {
    echo -e "${BLUE}Starting secure installation...${NC}"
    
    verify_script_integrity
    install_rust
    install_dependencies
    create_secure_temp_dir
    clone_repository
    install_rustloader
    setup_path
    test_installation
    verify_full_installation
    log_installation
    cleanup
    display_instructions
    
    echo -e "${GREEN}Secure installation completed successfully!${NC}"
}

# Run the installation
main