#!/bin/bash

# Improved Rustloader Installation Script
# This script provides a more robust installation process with better error handling,
# dependency validation, and user feedback

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Banner
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  ${CYAN}Rustloader Installation Script v1.1.0${BLUE}  ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"

# Function to print steps
print_step() {
    echo -e "\n${BLUE}[${CYAN}+${BLUE}] ${CYAN}$1${NC}"
}

# Function to print success messages
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Function to print info messages
print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

# Function to print warnings
print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Function to print errors
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to handle script interruption
handle_interrupt() {
    echo -e "\n${YELLOW}⚠ Installation interrupted. Cleaning up...${NC}"
    if [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
        echo -e "${GREEN}✓ Removed temporary files${NC}"
    fi
    echo -e "${YELLOW}⚠ Installation was not completed. Run the script again to start over.${NC}"
    exit 1
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to print spinner
function spinner() {
    local pid=$1
    local delay=0.1
    local spinstr='|/-\'
    while ps -p $pid > /dev/null; do
        local temp=${spinstr#?}
        printf " [%c]  " "$spinstr"
        local spinstr=$temp${spinstr%"$temp"}
        sleep $delay
        printf "\b\b\b\b\b\b"
    done
    printf "    \b\b\b\b"
}

# Set up trap for interruptions
trap handle_interrupt INT TERM

# Create a temporary directory for downloads
TMP_DIR=$(mktemp -d)
print_info "Created temporary directory: $TMP_DIR"

# Detect OS
print_step "Detecting Operating System"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="Linux"
    DISTRO="Unknown"
    
    # Detect distribution
    if command_exists apt || command_exists apt-get; then
        DISTRO="Debian"
    elif command_exists dnf; then
        DISTRO="Fedora"
    elif command_exists pacman; then
        DISTRO="Arch"
    fi
    
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macOS"
elif [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    OS="Windows"
else
    OS="Unknown"
fi

print_success "Detected $OS operating system $([ "$DISTRO" != "Unknown" ] && echo "($DISTRO)")"

# Check if running with sudo/root on Linux
if [[ "$OS" == "Linux" ]] && [[ $EUID -ne 0 ]]; then
    print_warning "Some operations might require sudo privileges."
    print_warning "If permission errors occur, consider running this script with sudo."
    echo -e "${YELLOW}Continue anyway? (y/n)${NC}"
    read -r cont
    if [[ ! $cont =~ ^[Yy]$ ]]; then
        print_error "Installation cancelled."
        exit 1
    fi
fi

# Check for existing installations
print_step "Checking for existing Rustloader installation"
if command_exists rustloader; then
    EXISTING_VERSION=$(rustloader --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "Unknown")
    if [ "$EXISTING_VERSION" != "Unknown" ]; then
        print_warning "Rustloader version $EXISTING_VERSION is already installed."
        echo -e "${YELLOW}Do you want to continue with the installation? This may update the existing installation. (y/n)${NC}"
        read -r cont
        if [[ ! $cont =~ ^[Yy]$ ]]; then
            print_info "Installation cancelled. Using existing installation."
            exit 0
        fi
    fi
else
    print_info "No existing Rustloader installation found."
fi

# Install Rust if not installed
print_step "Checking for Rust installation"
if ! command_exists cargo; then
    print_info "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > "$TMP_DIR/rustup-init.sh"
    
    # Verify the downloaded script (basic check)
    if grep -q "curl https://sh.rustup.rs -sSf | sh" "$TMP_DIR/rustup-init.sh"; then
        chmod +x "$TMP_DIR/rustup-init.sh"
        "$TMP_DIR/rustup-init.sh" -y
        source "$HOME/.cargo/env"
        print_success "Rust installed successfully."
    else
        print_error "Downloaded Rust installer validation failed. Installation aborted."
        exit 1
    fi
else
    RUST_VERSION=$(rustc --version | cut -d ' ' -f 2)
    print_success "Rust is already installed (version $RUST_VERSION)."
    
    # Check if rustup is available and update Rust
    if command_exists rustup; then
        print_info "Updating Rust using rustup..."
        rustup update &
        spinner $!
        print_success "Rust updated successfully."
    fi
fi

# Function to install and validate dependencies
install_dependencies() {
    print_step "Installing and validating dependencies"

    # Function to check dependency version
    check_dependency_version() {
        local dep_name=$1
        local min_version=$2
        local version_cmd=$3
        local version_regex=$4
        
        if command_exists "$dep_name"; then
            local version_output
            version_output=$($version_cmd 2>&1)
            local version
            version=$(echo "$version_output" | grep -oE "$version_regex" | head -n 1)
            
            if [ -n "$version" ]; then
                print_success "$dep_name found (version $version)"
                return 0
            else
                print_warning "Could not determine $dep_name version."
                return 1
            fi
        else
            print_warning "$dep_name not found."
            return 1
        fi
    }

    # Install yt-dlp
    if ! check_dependency_version "yt-dlp" "2023.7.6" "yt-dlp --version" "[0-9]+\.[0-9]+\.[0-9]+" || \
       [[ "$1" == "force" ]]; then
        print_info "Installing/updating yt-dlp..."

        case $OS in
            "Linux"|"macOS")
                # Try to use pip3 first, fall back to pip if needed
                if command_exists pip3; then
                    pip3 install --user --upgrade yt-dlp &
                    spinner $!
                elif command_exists pip; then
                    pip install --user --upgrade yt-dlp &
                    spinner $!
                else
                    print_error "Neither pip nor pip3 found. Please install Python pip first."
                    return 1
                fi
                ;;
            "Windows")
                if command_exists pip; then
                    pip install --user --upgrade yt-dlp &
                    spinner $!
                else
                    print_error "pip not found. Please install Python pip first."
                    return 1
                fi
                ;;
            *)
                print_error "Unsupported operating system: $OS"
                return 1
                ;;
        esac
        
        # Validate installation
        if command_exists yt-dlp; then
            YT_DLP_VERSION=$(yt-dlp --version 2>/dev/null || echo "Unknown")
            print_success "yt-dlp installed/updated successfully (version $YT_DLP_VERSION)."
        else
            print_warning "yt-dlp installation may have failed. Please check your PATH."
            
            # Try to find the binary location
            YT_DLP_PATH="$HOME/.local/bin/yt-dlp"
            if [ -f "$YT_DLP_PATH" ]; then
                print_info "Found yt-dlp at $YT_DLP_PATH but it's not in your PATH."
                print_info "Add the following to your .bashrc or .zshrc file:"
                echo "export PATH=\"\$HOME/.local/bin:\$PATH\""
            fi
        fi
    fi

    # Install ffmpeg
    if ! check_dependency_version "ffmpeg" "4.0.0" "ffmpeg -version" "ffmpeg version ([0-9]+\.[0-9]+(?:\.[0-9]+)?)" || \
       [[ "$1" == "force" ]]; then
        print_info "Installing/updating ffmpeg..."

        case $OS in
            "Linux")
                case $DISTRO in
                    "Debian")
                        sudo apt update && sudo apt install -y ffmpeg &
                        spinner $!
                        ;;
                    "Fedora")
                        sudo dnf install -y ffmpeg &
                        spinner $!
                        ;;
                    "Arch")
                        sudo pacman -Sy --noconfirm ffmpeg &
                        spinner $!
                        ;;
                    *)
                        print_warning "Unsupported Linux distribution. Please install ffmpeg manually."
                        print_info "Try: sudo apt install ffmpeg, sudo dnf install ffmpeg, or equivalent for your distribution."
                        ;;
                esac
                ;;
            "macOS")
                if command_exists brew; then
                    brew install ffmpeg &
                    spinner $!
                else
                    print_warning "Homebrew not found. Installing Homebrew..."
                    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
                    if command_exists brew; then
                        brew install ffmpeg &
                        spinner $!
                    else
                        print_error "Failed to install Homebrew. Please install ffmpeg manually."
                        return 1
                    fi
                fi
                ;;
            "Windows")
                if command_exists choco; then
                    choco install ffmpeg -y &
                    spinner $!
                else
                    print_warning "Chocolatey not found. Please install ffmpeg manually."
                    print_info "Visit: https://ffmpeg.org/download.html"
                    # Continue installation, ffmpeg is not strictly required
                fi
                ;;
            *)
                print_error "Unsupported operating system: $OS"
                return 1
                ;;
        esac
        
        # Validate installation
        if command_exists ffmpeg; then
            FFMPEG_VERSION=$(ffmpeg -version | grep -oE "ffmpeg version ([0-9]+\.[0-9]+(?:\.[0-9]+)?)" | sed 's/ffmpeg version //')
            print_success "ffmpeg installed/updated successfully (version $FFMPEG_VERSION)."
        else
            print_warning "ffmpeg installation may have failed. Continuing without it."
            print_warning "Some Rustloader features requiring ffmpeg will not work."
        fi
    fi

    return 0
}

# Install dependencies
install_dependencies

# Clone and build Rustloader
print_step "Downloading and building Rustloader"

# Clone the repository
print_info "Cloning Rustloader repository..."
git clone https://github.com/ibra2000sd/rustloader.git "$TMP_DIR/rustloader" &
spinner $!

if [ ! -d "$TMP_DIR/rustloader" ]; then
    print_error "Failed to clone Rustloader repository."
    exit 1
fi

# Build Rustloader
print_info "Building Rustloader..."
cd "$TMP_DIR/rustloader"

# Build with debug output in case of errors
cargo build --release || {
    print_error "Build failed. Checking for common issues..."
    
    # Check for common issues
    if ! command_exists cc; then
        print_error "C compiler not found. Please install build-essential (Linux) or Xcode Command Line Tools (macOS)."
    fi
    
    if ! command_exists pkg-config; then
        print_error "pkg-config not found. Please install it with your package manager."
    fi
    
    exit 1
}

# Check if build was successful
if [ ! -f "$TMP_DIR/rustloader/target/release/rustloader" ]; then
    print_error "Build failed. Please check error messages above."
    exit 1
fi

# Install the binary
print_step "Installing Rustloader binary"

# Create cargo bin directory if it doesn't exist
mkdir -p "$HOME/.cargo/bin"

# Copy to cargo bin directory
cp "$TMP_DIR/rustloader/target/release/rustloader" "$HOME/.cargo/bin/rustloader"

# Make executable
chmod +x "$HOME/.cargo/bin/rustloader"

print_success "Rustloader binary installed successfully at $HOME/.cargo/bin/rustloader"

# Set up PATH if needed
print_step "Setting up PATH"

# Check if ~/.cargo/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
    print_info "Adding ~/.cargo/bin to PATH..."
    
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
    print_success "Added ~/.cargo/bin to PATH in $PROFILE_FILE"
    print_info "Please run 'source $PROFILE_FILE' or restart your terminal to apply changes."
else
    print_success "~/.cargo/bin is already in PATH."
fi

# Create download directories
print_step "Creating download directories"

mkdir -p "$HOME/Downloads/rustloader/videos"
mkdir -p "$HOME/Downloads/rustloader/audio"
print_success "Created download directories in $HOME/Downloads/rustloader/"

# Clean up
print_step "Cleaning up"
cd "$HOME"
rm -rf "$TMP_DIR"
print_success "Removed temporary files"

# Test the installation
print_step "Testing Rustloader installation"

export PATH="$HOME/.cargo/bin:$PATH"
if command_exists rustloader; then
    RUSTLOADER_VERSION=$(rustloader --version 2>/dev/null)
    print_success "Rustloader is installed and in PATH."
    print_success "Version: $RUSTLOADER_VERSION"
else
    print_warning "Rustloader is not in PATH. You may need to restart your terminal or run: export PATH=\"\$HOME/.cargo/bin:\$PATH\""
    print_info "You can verify the installation manually by running: $HOME/.cargo/bin/rustloader --version"
fi

# Display completion message
print_step "Installation completed!"

echo -e "${GREEN}╔═════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Rustloader has been successfully installed!    ║${NC}"
echo -e "${GREEN}╚═════════════════════════════════════════════════╝${NC}"

echo -e "\n${CYAN}Basic Usage:${NC}"
echo -e "  ${BLUE}rustloader${NC} [URL] [OPTIONS]"

echo -e "\n${CYAN}Examples:${NC}"
echo -e "  ${BLUE}rustloader${NC} https://www.youtube.com/watch?v=dQw4w9WgXcQ                  # Download video"
echo -e "  ${BLUE}rustloader${NC} https://www.youtube.com/watch?v=dQw4w9WgXcQ --format mp3     # Download audio"
echo -e "  ${BLUE}rustloader${NC} https://www.youtube.com/watch?v=dQw4w9WgXcQ --quality 720    # Set quality"
echo -e "  ${BLUE}rustloader${NC} --help                                                        # Show all options"

echo -e "\n${CYAN}OS-Specific Notes:${NC}"
case $OS in
    "Linux")
        echo -e "  • If ${BLUE}rustloader${NC} is not found, run: ${GREEN}source $PROFILE_FILE${NC}"
        echo -e "  • For system-wide installation: ${GREEN}sudo cp ~/.cargo/bin/rustloader /usr/local/bin/${NC}"
        ;;
    "macOS")
        echo -e "  • If you encounter permissions issues, run: ${GREEN}brew doctor${NC}"
        echo -e "  • Make sure your PATH includes: ${GREEN}$HOME/.cargo/bin${NC}"
        ;;
    "Windows")
        echo -e "  • You may need to restart your terminal or computer for PATH changes to take effect"
        echo -e "  • Make sure ${GREEN}%USERPROFILE%\\.cargo\\bin${NC} is in your PATH"
        ;;
esac

echo -e "\n${CYAN}Need help or have questions?${NC}"
echo -e "  • Visit: ${GREEN}https://rustloader.com/docs${NC}"
echo -e "  • Report issues: ${GREEN}https://github.com/ibra2000sd/rustloader/issues${NC}"

echo -e "\n${YELLOW}Enjoying Rustloader? Consider upgrading to Pro for:${NC}"
echo -e "  • ${GREEN}4K/8K video quality downloads${NC}"
echo -e "  • ${GREEN}High-fidelity audio (320kbps, FLAC)${NC}"
echo -e "  • ${GREEN}Unlimited downloads${NC}"
echo -e "  • ${GREEN}Multi-threaded downloads for maximum speed${NC}"
echo -e "  • ${GREEN}Priority updates and support${NC}"

echo -e "\n${BLUE}Thank you for installing Rustloader!${NC}"