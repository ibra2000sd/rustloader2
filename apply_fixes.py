#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path
from typing import Dict, List

MAX_CHANGES_PER_FILE = 10  
MAX_CHANGE_PERCENTAGE = 20  

def log_info(message: str) -> None:
    """Log an informational message"""
    print(f"[INFO] {message}")

def log_warning(message: str) -> None:
    """Log a warning message"""
    print(f"[WARNING] {message}", file=sys.stderr)

def log_error(message: str) -> None:
    """Log an error message"""
    print(f"[ERROR] {message}", file=sys.stderr)

def load_fixes(fixes_file: str) -> List[Dict]:
    """Load the fixes JSON file"""
    try:
        with open(fixes_file, "r", encoding="utf-8") as f:
            return json.load(f)
    except json.JSONDecodeError:
        log_error(f"❌ Failed to load {fixes_file}: Invalid JSON format.")
        sys.exit(1)
    except FileNotFoundError:
        log_error(f"❌ File {fixes_file} not found.")
        sys.exit(1)

def apply_fixes(fixes: List[Dict]) -> bool:
    """Apply the fixes to the specified files"""
    changes_applied = 0
    files_modified = 0

    for fix in fixes:
        file_path = fix.get("file")
        changes = fix.get("changes", [])

        if not file_path or not Path(file_path).exists():
            log_warning(f"⚠️ Skipping {file_path}: File does not exist.")
            continue

        if len(changes) > MAX_CHANGES_PER_FILE:
            log_warning(f"⚠️ Skipping {file_path}: Too many changes ({MAX_CHANGES_PER_FILE} limit exceeded).")
            continue

        try:
            with open(file_path, "r", encoding="utf-8") as f:
                content = f.read()

            original_length = len(content)
            modified_content = content
            file_changes = 0

            for change in changes:
                original = change.get("original")
                fixed = change.get("fixed")

                if not original or not fixed or original not in modified_content:
                    log_warning(f"⚠️ Skipping a change in {file_path}: Original code not found.")
                    continue

                modified_content = modified_content.replace(original, fixed)
                file_changes += 1

            if len(modified_content) > 0:
                diff_percentage = abs(len(modified_content) - original_length) / len(modified_content) * 100
                if diff_percentage > MAX_CHANGE_PERCENTAGE:
                    log_warning(f"⚠️ Skipping {file_path}: Change percentage too high ({diff_percentage:.2f}%).")
                    continue

            if modified_content != content and file_changes > 0:
                with open(file_path, "w", encoding="utf-8") as f:
                    f.write(modified_content)
                log_info(f"✅ Applied {file_changes} change(s) in {file_path}.")
                changes_applied += file_changes
                files_modified += 1

        except Exception as e:
            log_error(f"❌ Error processing {file_path}: {e}")

    log_info(f"✨ Total: {changes_applied} change(s) applied to {files_modified} file(s).")
    return changes_applied > 0

def main():
    """Run the fix application script"""
    if len(sys.argv) < 2:
        log_error("❌ Usage: python apply_fixes.py <fixes_file>")
        sys.exit(1)

    fixes_file = sys.argv[1]
    fixes = load_fixes(fixes_file)
    success = apply_fixes(fixes)
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
