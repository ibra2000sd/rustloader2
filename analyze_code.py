#!/usr/bin/env python3
import os
import json
import requests
import time
import re
import sys
from pathlib import Path
from typing import Dict, List, Optional, Union, Any

# Configuration
API_URL = "https://api.anthropic.com/v1/messages"
API_KEY = os.getenv("CLAUDE_API_KEY")
MODEL = "claude-3-7-sonnet-20250219"
MAX_TOKENS = 4000
MAX_RETRIES = 3
RETRY_DELAY = 5  # seconds
RETRY_STATUS_CODES = [429, 500, 502, 503, 504]  # Include rate limiting (429)

def log_info(message: str) -> None:
    """Log an informational message with timestamp."""
    print(f"[INFO] {message}")

def log_warning(message: str) -> None:
    """Log a warning message with timestamp."""
    print(f"[WARNING] {message}", file=sys.stderr)

def log_error(message: str) -> None:
    """Log an error message with timestamp."""
    print(f"[ERROR] {message}", file=sys.stderr)

def read_file(filename: str) -> str:
    """Safely read a file if it exists, otherwise return an empty string."""
    try:
        path = Path(filename)
        if path.exists():
            return path.read_text(encoding="utf-8")
        return ""
    except Exception as e:
        log_warning(f"Failed to read {filename}: {e}")
        return ""

def write_file(filename: str, content: str) -> bool:
    """Write content to a file safely."""
    try:
        path = Path(filename)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return True
    except Exception as e:
        log_error(f"Failed to write to {filename}: {e}")
        return False

def write_json(filename: str, data: Any) -> bool:
    """Write JSON data to a file safely."""
    try:
        path = Path(filename)
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
        return True
    except Exception as e:
        log_error(f"Failed to write JSON to {filename}: {e}")
        return False

def extract_fixes_json(text: str) -> Optional[str]:
    """Extract the fixes JSON from between <FIXES> tags."""
    pattern = r"<FIXES>\s*([\s\S]*?)\s*</FIXES>"
    match = re.search(pattern, text)
    if match:
        return match.group(1).strip()
    return None

def filter_clippy_warnings(clippy_data: List[Dict]) -> List[str]:
    """Extract unique file paths from Clippy warnings."""
    file_paths = set()
    
    for item in clippy_data:
        if "message" in item and "spans" in item.get("message", {}):
            for span in item["message"].get("spans", []):
                if "file_name" in span:
                    file_path = span["file_name"]
                    if Path(file_path).exists():
                        file_paths.add(file_path)
    
    return list(file_paths)

def prepare_code_content(clippy_data: Optional[List[Dict]] = None) -> str:
    """Prepare code content for analysis, prioritizing files with Clippy warnings."""
    files_with_warnings = []
    
    # Try to extract files with warnings from Clippy results
    if clippy_data:
        files_with_warnings = filter_clippy_warnings(clippy_data)
    
    # If no Clippy warnings or clippy_data is empty, fall back to a basic search
    if not files_with_warnings:
        log_info("No Clippy warnings found, falling back to finding Rust files")
        # Find up to 5 Rust files
        rust_files = list(Path("src").glob("**/*.rs"))[:5] if Path("src").exists() else []
        files_with_warnings = [str(f) for f in rust_files]
    
    # If we still don't have files, give up
    if not files_with_warnings:
        log_warning("No Rust files found to analyze")
        return ""
    
    # Build content from files
    content_parts = []
    for file_path in files_with_warnings:
        try:
            file_content = Path(file_path).read_text(encoding="utf-8")
            content_parts.append(f"File: {file_path}\n```rust\n{file_content}\n```\n\n")
        except Exception as e:
            log_warning(f"Failed to read {file_path}: {e}")
    
    return "".join(content_parts)

def call_claude_api(payload: Dict) -> Optional[Dict]:
    """Call Claude API with retry logic and error handling."""
    headers = {
        "x-api-key": API_KEY,
        "anthropic-version": "2023-06-01",
        "content-type": "application/json",
    }
    
    for attempt in range(MAX_RETRIES):
        try:
            log_info(f"Calling Claude API (attempt {attempt+1}/{MAX_RETRIES})")
            response = requests.post(API_URL, headers=headers, json=payload, timeout=120)
            
            # Handle rate limiting and server errors with retries
            if response.status_code in RETRY_STATUS_CODES:
                retry_after = int(response.headers.get("Retry-After", RETRY_DELAY))
                log_warning(f"API error {response.status_code}, retrying in {retry_after}s... ({attempt+1}/{MAX_RETRIES})")
                time.sleep(retry_after)
                continue
                
            # Handle other errors
            if response.status_code != 200:
                log_error(f"API request failed with status code {response.status_code}")
                log_error(f"Response: {response.text}")
                return None
            
            return response.json()
            
        except requests.exceptions.RequestException as e:
            log_error(f"Request error: {e}")
            if attempt < MAX_RETRIES - 1:
                log_info(f"Retrying in {RETRY_DELAY}s... ({attempt+1}/{MAX_RETRIES})")
                time.sleep(RETRY_DELAY)
            else:
                log_error(f"Max retries reached ({MAX_RETRIES})")
                return None
    
    return None

def main() -> int:
    """Main function to orchestrate the code analysis process."""
    if not API_KEY:
        log_error("CLAUDE_API_KEY environment variable is not set")
        return 1
    
    # Read input files
    log_info("Reading input files")
    clippy_raw = read_file("clippy-results.json")
    audit_raw = read_file("audit-results.json")
    
    # Parse Clippy results if available
    clippy_data = None
    if clippy_raw:
        try:
            clippy_data = json.loads(clippy_raw)
            log_info(f"Found {len(clippy_data)} Clippy warnings")
        except json.JSONDecodeError:
            log_warning("Failed to parse Clippy results, treating as empty")
    
    # Prepare code content for analysis
    log_info("Preparing code content for analysis")
    code_content = prepare_code_content(clippy_data)
    
    if not code_content:
        log_error("No code content to analyze")
        write_file("claude-analysis-report.md", "No code content to analyze")
        print("fixes_available=false")
        return 1
    
    # Prepare API request payload
    request_payload = {
        "model": MODEL,
        "max_tokens": MAX_TOKENS,
        "temperature": 0.2,
        "messages": [
            {
                "role": "user",
                "content": f"""You are a Rust code improvement expert. I need you to analyze the following Rust code, Clippy warnings, and Cargo Audit results, then provide specific fixes.

Clippy Results:
{clippy_raw}

Audit Results:
{audit_raw}

Code Files:
{code_content}

Please provide the following in your response:

1. A markdown report summarizing the issues found
2. For each issue that can be automatically fixed, provide a JSON object in the following format surrounded by <FIXES> tags:

<FIXES>
[
  {{
    "file": "path/to/file.rs",
    "changes": [
      {{
        "original": "problematic code snippet",
        "fixed": "corrected code snippet"
      }}
    ]
  }}
]
</FIXES>

Make sure the changes are minimal, correct, and maintain the same functionality. Focus on fixes for security issues, performance problems, and code quality issues. The JSON must be valid and the code snippets must be exact matches of what is in the files."""
            }
        ]
    }
    
    # Call the Claude API
    response_json = call_claude_api(request_payload)
    
    if not response_json:
        log_error("Failed to get response from Claude API")
        print("fixes_available=false")
        return 1
    
    # Save the full response
    write_json("claude-full-response.json", response_json)
    
    # Extract and process the response text
    if "content" in response_json and response_json["content"]:
        response_text = response_json["content"][0]["text"]
        write_file("claude-analysis-report.md", response_text)
        
        # Extract fixes JSON
        fixes_json = extract_fixes_json(response_text)
        if fixes_json:
            try:
                fixes_data = json.loads(fixes_json)
                write_json("fixes.json", fixes_data)
                log_info("✓ Fixes extracted successfully!")
                print("fixes_available=true")
                return 0
            except json.JSONDecodeError as e:
                log_error(f"Error parsing fixes JSON: {e}")
                # Write the problematic JSON for debugging
                write_file("invalid-fixes.json", fixes_json)
                print("fixes_available=false")
                return 1
        else:
            log_info("No fixes found in Claude's response")
            print("fixes_available=false")
            return 0
    else:
        log_error("Empty or invalid response from Claude API")
        print("fixes_available=false")
        return 1

if __name__ == "__main__":
    sys.exit(main())