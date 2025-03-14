#!/bin/bash

# Rustloader Uninstallation Script
# This script removes Rustloader but keeps yt-dlp and ffmpeg intact

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}    Rustloader Uninstallation Script    ${NC}"
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

# Check for sudo on Linux
if [[ "$OS" == "Linux" ]] && [[ $EUID -ne 0 ]]; then
    echo -e "${YELLOW}Note: Some operations might require sudo privileges.${NC}"
    echo -e "${YELLOW}If permission errors occur, consider running this script with sudo.${NC}"
fi

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to handle script interruption
handle_interrupt() {
    echo -e "\n${YELLOW}Uninstallation interrupted. Some components may not have been completely removed.${NC}"
    exit 1
}

# Set up trap for interruptions
trap handle_interrupt INT TERM

# Get confirmation for data removal
get_confirmation() {
    local message=$1
    local default=${2:-n}
    
    if [[ "$default" == "y" ]]; then
        read -p "$message (Y/n): " CONFIRM
        [[ -z "$CONFIRM" || "$CONFIRM" =~ ^[Yy] ]]
    else
        read -p "$message (y/N): " CONFIRM
        [[ "$CONFIRM" =~ ^[Yy]$ ]]
    fi
}

# Uninstall rustloader binary
uninstall_binary() {
    echo -e "${YELLOW}Checking for Rustloader binary...${NC}"
    
    local binary_path=""
    local binary_found=false
    
    # Check common installation locations based on OS
    if command_exists rustloader; then
        binary_path=$(which rustloader)
        binary_found=true
    elif [[ -f "$HOME/.cargo/bin/rustloader" ]]; then
        binary_path="$HOME/.cargo/bin/rustloader"
        binary_found=true
    elif [[ -f "/usr/local/bin/rustloader" ]]; then
        binary_path="/usr/local/bin/rustloader"
        binary_found=true
    fi
    
    if [[ "$binary_found" == true ]]; then
        echo -e "${GREEN}Found Rustloader binary at: $binary_path${NC}"
        
        # Handle removal based on location
        if [[ "$binary_path" == "$HOME/.cargo/bin/rustloader" ]]; then
            echo -e "${YELLOW}Removing Rustloader binary from cargo directory...${NC}"
            if rm -f "$binary_path"; then
                echo -e "${GREEN}Successfully removed Rustloader binary${NC}"
            else
                echo -e "${RED}Failed to remove Rustloader binary. Please check permissions.${NC}"
                return 1
            fi
        elif [[ "$binary_path" == "/usr/local/bin/rustloader" ]]; then
            echo -e "${YELLOW}Rustloader binary is installed in system directory${NC}"
            echo -e "${YELLOW}This may require sudo privileges to remove${NC}"
            
            if get_confirmation "Remove Rustloader from $binary_path?"; then
                if command_exists sudo; then
                    sudo rm -f "$binary_path"
                    echo -e "${GREEN}Successfully removed Rustloader binary${NC}"
                else
                    if rm -f "$binary_path" 2>/dev/null; then
                        echo -e "${GREEN}Successfully removed Rustloader binary${NC}"
                    else
                        echo -e "${RED}Failed to remove Rustloader binary. Please check permissions.${NC}"
                        echo -e "${YELLOW}Try running: sudo rm -f $binary_path${NC}"
                        return 1
                    fi
                fi
            else
                echo -e "${YELLOW}Skipping removal of system binary${NC}"
            fi
        else
            echo -e "${YELLOW}Removing Rustloader binary from $binary_path...${NC}"
            if rm -f "$binary_path"; then
                echo -e "${GREEN}Successfully removed Rustloader binary${NC}"
            else
                echo -e "${RED}Failed to remove Rustloader binary. Please check permissions.${NC}"
                return 1
            fi
        fi
    elif [[ -f "$(cargo install --list | grep -o "^rustloader.*")" ]]; then
        echo -e "${YELLOW}Rustloader found in Cargo registry. Uninstalling...${NC}"
        if cargo uninstall rustloader; then
            echo -e "${GREEN}Successfully removed Rustloader via Cargo${NC}"
        else
            echo -e "${RED}Failed to uninstall Rustloader via Cargo${NC}"
            return 1
        fi
    else
        echo -e "${YELLOW}Rustloader binary not found in PATH or common locations${NC}"
        
        # Try removing via cargo as a last resort
        if command_exists cargo; then
            echo -e "${YELLOW}Attempting to uninstall via Cargo...${NC}"
            if cargo uninstall rustloader 2>/dev/null; then
                echo -e "${GREEN}Successfully removed Rustloader via Cargo${NC}"
            else
                echo -e "${YELLOW}Rustloader not found in Cargo registry${NC}"
                echo -e "${YELLOW}If you installed Rustloader manually to a custom location,${NC}"
                echo -e "${YELLOW}you'll need to remove it manually.${NC}"
            fi
        fi
    fi
}

# Remove configuration files
remove_config_files() {
    echo -e "${YELLOW}Checking for configuration files...${NC}"
    
    local config_dir=""
    
    # Determine config directory based on OS
    if [[ "$OS" == "Linux" ]]; then
        config_dir="$HOME/.config/rustloader"
    elif [[ "$OS" == "macOS" ]]; then
        config_dir="$HOME/Library/Application Support/rustloader"
    elif [[ "$OS" == "Windows" ]]; then
        config_dir="$APPDATA/rustloader"
    else
        config_dir="$HOME/.config/rustloader"
    fi
    
    if [[ -d "$config_dir" ]]; then
        echo -e "${GREEN}Found configuration directory: $config_dir${NC}"
        
        if get_confirmation "Remove configuration directory and all settings?"; then
            if rm -rf "$config_dir"; then
                echo -e "${GREEN}Successfully removed configuration directory${NC}"
            else
                echo -e "${RED}Failed to remove configuration directory. Please check permissions.${NC}"
            fi
        else
            echo -e "${YELLOW}Keeping configuration directory${NC}"
        fi
    else
        echo -e "${YELLOW}Configuration directory not found${NC}"
    fi
    
    # Check for alternative locations, just to be thorough
    local alt_config_dir="$HOME/.rustloader"
    if [[ -d "$alt_config_dir" ]]; then
        echo -e "${GREEN}Found alternative configuration directory: $alt_config_dir${NC}"
        
        if get_confirmation "Remove this configuration directory as well?"; then
            if rm -rf "$alt_config_dir"; then
                echo -e "${GREEN}Successfully removed alternative configuration directory${NC}"
            else
                echo -e "${RED}Failed to remove alternative configuration directory. Please check permissions.${NC}"
            fi
        else
            echo -e "${YELLOW}Keeping alternative configuration directory${NC}"
        fi
    fi
}

# Remove data files
remove_data_files() {
    echo -e "${YELLOW}Checking for data files...${NC}"
    
    local data_dir=""
    
    # Determine data directory based on OS
    if [[ "$OS" == "Linux" ]]; then
        data_dir="$HOME/.local/share/rustloader"
    elif [[ "$OS" == "macOS" ]]; then
        data_dir="$HOME/Library/Application Support/rustloader"
    elif [[ "$OS" == "Windows" ]]; then
        data_dir="$APPDATA/rustloader"
    else
        data_dir="$HOME/.local/share/rustloader"
    fi
    
    if [[ -d "$data_dir" ]]; then
        echo -e "${GREEN}Found data directory: $data_dir${NC}"
        
        if get_confirmation "Remove all program data (counters, cache, etc.)?"; then
            if rm -rf "$data_dir"; then
                echo -e "${GREEN}Successfully removed data directory${NC}"
            else
                echo -e "${RED}Failed to remove data directory. Please check permissions.${NC}"
            fi
        else
            echo -e "${YELLOW}Keeping data directory${NC}"
        fi
    else
        echo -e "${YELLOW}Data directory not found${NC}"
    fi
}

# Remove downloaded content
remove_downloads() {
    echo -e "${YELLOW}Checking for downloaded content...${NC}"
    
    local download_dirs=()
    
    # Add default download location
    download_dirs+=("$HOME/Downloads/rustloader")
    
    # Check if custom download location was specified
    local custom_dir_file="$HOME/.config/rustloader/download_path"
    if [[ -f "$custom_dir_file" ]]; then
        custom_dir=$(cat "$custom_dir_file")
        if [[ -n "$custom_dir" && -d "$custom_dir" ]]; then
            download_dirs+=("$custom_dir")
        fi
    fi
    
    # Check and remove each potential download directory
    for dir in "${download_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            echo -e "${GREEN}Found download directory: $dir${NC}"
            
            # Show directory size before asking for confirmation
            local dir_size=""
            if command_exists du; then
                dir_size=$(du -sh "$dir" | cut -f1)
                echo -e "${YELLOW}Directory size: $dir_size${NC}"
            fi
            
            if get_confirmation "Do you want to delete downloaded files?"; then
                if rm -rf "$dir"; then
                    echo -e "${GREEN}Successfully removed downloaded content${NC}"
                else
                    echo -e "${RED}Failed to remove downloaded content. Please check permissions.${NC}"
                fi
            else
                echo -e "${YELLOW}Keeping downloaded content${NC}"
            fi
        fi
    done
    
    if [[ ${#download_dirs[@]} -eq 0 || ! -d "${download_dirs[0]}" ]]; then
        echo -e "${YELLOW}No download directories found${NC}"
    fi
}

# Remove license file (if it exists)
remove_license() {
    echo -e "${YELLOW}Checking for license files...${NC}"
    
    local license_found=false
    local license_files=()
    
    # Add all potential license locations based on OS
    if [[ "$OS" == "Linux" ]]; then
        license_files+=("$HOME/.config/rustloader/license.dat")
        license_files+=("$HOME/.local/share/rustloader/license.dat")
    elif [[ "$OS" == "macOS" ]]; then
        license_files+=("$HOME/Library/Application Support/rustloader/license.dat")
        license_files+=("$HOME/Library/Preferences/rustloader/license.dat")
    elif [[ "$OS" == "Windows" ]]; then
        license_files+=("$APPDATA/rustloader/license.dat")
    else
        license_files+=("$HOME/.config/rustloader/license.dat")
    fi
    
    # Check and remove each potential license file
    for file in "${license_files[@]}"; do
        if [[ -f "$file" ]]; then
            echo -e "${GREEN}Found license file: $file${NC}"
            license_found=true
            
            if get_confirmation "Remove license file?"; then
                if rm -f "$file"; then
                    echo -e "${GREEN}Successfully removed license file${NC}"
                else
                    echo -e "${RED}Failed to remove license file. Please check permissions.${NC}"
                fi
            else
                echo -e "${YELLOW}Keeping license file${NC}"
            fi
        fi
    done
    
    if [[ "$license_found" != true ]]; then
        echo -e "${YELLOW}No license files found${NC}"
    fi
}

# Check for leftover files
check_leftover_files() {
    echo -e "${BLUE}Checking for any leftover files...${NC}"
    
    local leftover_found=false
    local leftover_files=()
    
    # Check common locations for leftover files
    potential_locations=(
        "$HOME/.rustloader"
        "$HOME/.config/rustloader"
        "$HOME/.local/share/rustloader"
        "$HOME/Library/Application Support/rustloader"
        "$HOME/Library/Preferences/rustloader"
        "$APPDATA/rustloader"
    )
    
    for location in "${potential_locations[@]}"; do
        if [[ -e "$location" ]]; then
            leftover_files+=("$location")
            leftover_found=true
        fi
    done
    
    if [[ "$leftover_found" == true ]]; then
        echo -e "${YELLOW}Found leftover files:${NC}"
        for file in "${leftover_files[@]}"; do
            echo -e "  - $file"
        done
        
        if get_confirmation "Remove all leftover files?"; then
            for file in "${leftover_files[@]}"; do
                if rm -rf "$file"; then
                    echo -e "${GREEN}Removed: $file${NC}"
                else
                    echo -e "${RED}Failed to remove: $file${NC}"
                fi
            done
        else
            echo -e "${YELLOW}Keeping leftover files${NC}"
        fi
    else
        echo -e "${GREEN}No leftover files found${NC}"
    fi
}

# Display completion message with instructions
display_completion_message() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}    Rustloader has been uninstalled    ${NC}"
    echo -e "${GREEN}========================================${NC}"
    
    # Mention that yt-dlp and ffmpeg were kept
    echo -e "${YELLOW}Note: yt-dlp and ffmpeg were kept as requested${NC}"
    echo -e "${YELLOW}These dependencies are still installed and can be used by other applications${NC}"
    
    # Provide instructions for manual cleanup if needed
    echo -e "${BLUE}If you want to remove yt-dlp and ffmpeg as well:${NC}"
    
    case $OS in
        "Linux")
            echo -e "  - To remove yt-dlp: ${GREEN}pip uninstall yt-dlp${NC}"
            echo -e "  - To remove ffmpeg: ${GREEN}sudo apt remove ffmpeg${NC} (or equivalent for your distribution)"
            ;;
        "macOS")
            echo -e "  - To remove yt-dlp and ffmpeg: ${GREEN}brew uninstall yt-dlp ffmpeg${NC}"
            ;;
        "Windows")
            echo -e "  - To remove yt-dlp: ${GREEN}pip uninstall yt-dlp${NC}"
            echo -e "  - To remove ffmpeg: Use Windows Add/Remove Programs or ${GREEN}choco uninstall ffmpeg${NC}"
            ;;
        *)
            echo -e "  - To remove yt-dlp: ${GREEN}pip uninstall yt-dlp${NC}"
            echo -e "  - To remove ffmpeg: Use your system's package manager"
            ;;
    esac
    
    echo -e "\n${YELLOW}Thank you for using Rustloader!${NC}"
}

# Main uninstallation function
main() {
    echo -e "${BLUE}Starting uninstallation process...${NC}"
    
    # Perform each uninstallation step
    uninstall_binary
    remove_config_files
    remove_data_files
    remove_downloads
    remove_license
    check_leftover_files
    display_completion_message
}

# Run the main function
main