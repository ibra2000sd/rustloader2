#!/usr/bin/env python3
"""
analyze_code.py - Analyzes Rust code with Claude API

This script:
1. Reads output from Clippy and Cargo Audit
2. Sends the data to Claude API for analysis
3. Processes the response to extract suggested fixes
4. Generates a report and saves the fixes for later application
"""

import json
import os
import sys
import time
from pathlib import Path
import requests
from anthropic import Anthropic, HUMAN_PROMPT, AI_PROMPT

MAX_RETRIES = 3
RETRY_DELAY = 2  # seconds

def read_file_if_exists(filepath):
    """Read a file if it exists, otherwise return empty string."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            return f.read()
    except Exception as e:
        print(f"Warning: Could not read file {filepath}: {e}")
        return ""

def get_file_content(path, max_size=100000):
    """Read file content with size limitation."""
    try:
        if os.path.getsize(path) > max_size:
            return f"File too large to include ({os.path.getsize(path)} bytes)"
        
        with open(path, 'r', encoding='utf-8') as f:
            return f.read()
    except Exception as e:
        return f"Error reading file: {e}"

def get_changed_files():
    """Get list of changed files in the PR or commit."""
    event_name = os.environ.get('EVENT_NAME')
    
    if event_name == 'pull_request':
        # For PRs, use GitHub API to get changed files
        pr_number = os.environ.get('PR_NUMBER')
        token = os.environ.get('GITHUB_TOKEN')
        repo = os.environ.get('REPO_NAME')
        
        if not all([pr_number, token, repo]):
            print("Missing environment variables for PR file detection")
            return []
        
        url = f"https://api.github.com/repos/{repo}/pulls/{pr_number}/files"
        headers = {
            'Authorization': f'token {token}',
            'Accept': 'application/vnd.github.v3+json'
        }
        
        try:
            response = requests.get(url, headers=headers)
            response.raise_for_status()
            files = [file['filename'] for file in response.json() 
                    if file['filename'].endswith('.rs')]
            return files
        except Exception as e:
            print(f"Error fetching PR files: {e}")
            return []
    else:
        # For push events, use git diff
        try:
            import subprocess
            result = subprocess.run(
                ['git', 'diff', '--name-only', 'HEAD~1', 'HEAD'], 
                capture_output=True, text=True, check=True
            )
            return [f for f in result.stdout.split('\n') if f.endswith('.rs')]
        except Exception as e:
            print(f"Error getting changed files: {e}")
            return []

def format_clippy_output(clippy_data):
    """Format Clippy JSON data into readable format."""
    formatted_output = []
    
    try:
        clippy_issues = []
        for line in clippy_data.strip().split('\n'):
            if not line.strip():
                continue
            try:
                data = json.loads(line)
                if data.get('reason') == 'compiler-message' and data.get('message'):
                    message = data['message']
                    if message.get('level') in ['warning', 'error'] and 'rendered' in message:
                        clippy_issues.append(message)
            except json.JSONDecodeError:
                continue
        
        if not clippy_issues:
            return "No Clippy issues found."
        
        for issue in clippy_issues:
            formatted_output.append(issue.get('rendered', 'No rendered output'))
        
        return "\n\n".join(formatted_output)
    except Exception as e:
        return f"Error parsing Clippy output: {e}\n{clippy_data[:500]}"

def format_audit_output(audit_data):
    """Format Cargo Audit JSON data into readable format."""
    try:
        audit_json = json.loads(audit_data)
        if not audit_json.get('vulnerabilities', {}).get('found', False):
            return "No security vulnerabilities found."
        
        formatted_output = ["# Security Vulnerabilities\n"]
        
        for vuln in audit_json.get('vulnerabilities', {}).get('list', []):
            formatted_output.append(f"## {vuln.get('package', {}).get('name', 'Unknown')} - {vuln.get('advisory', {}).get('id', 'Unknown ID')}")
            formatted_output.append(f"- **Version**: {vuln.get('package', {}).get('version', 'Unknown')}")
            formatted_output.append(f"- **Title**: {vuln.get('advisory', {}).get('title', 'Unknown')}")
            formatted_output.append(f"- **URL**: {vuln.get('advisory', {}).get('url', 'No URL')}")
            formatted_output.append(f"- **Severity**: {vuln.get('advisory', {}).get('severity', 'Unknown')}")
            formatted_output.append("")
        
        return "\n".join(formatted_output)
    except json.JSONDecodeError:
        return f"Error parsing audit output as JSON: {audit_data[:500]}"
    except Exception as e:
        return f"Error formatting audit output: {e}"

def get_file_samples(files, max_files=5, max_size=50000):
    """Get content from a sample of rust files for context."""
    if not files:
        all_rs_files = [f for f in Path('.').glob('**/*.rs') 
                       if not str(f).startswith('./target/')]
        # Prioritize main.rs, lib.rs, and short files
        main_files = [f for f in all_rs_files if f.name in ['main.rs', 'lib.rs']]
        other_files = [f for f in all_rs_files if f.name not in ['main.rs', 'lib.rs']]
        
        # Sort other files by size to include smaller files first
        other_files.sort(key=lambda f: f.stat().st_size)
        
        # Combine lists with main files first
        files = main_files + other_files
    
    samples = []
    total_size = 0
    file_count = 0
    
    for file_path in files:
        if file_count >= max_files or total_size >= max_size:
            break
            
        path_str = str(file_path)
        if not path_str.endswith('.rs'):
            continue
            
        try:
            size = os.path.getsize(file_path)
            if size > max_size / 2:  # Skip very large files
                continue
                
            content = get_file_content(file_path)
            if content:
                samples.append(f"### File: {path_str}\n```rust\n{content}\n```\n")
                total_size += size
                file_count += 1
        except Exception as e:
            print(f"Error reading {file_path}: {e}")
    
    return "\n\n".join(samples)

def call_claude_api(prompt, api_key):
    """Call Claude API and return the response."""
    client = Anthropic(api_key=api_key)
    
    for attempt in range(MAX_RETRIES):
        try:
            response = client.messages.create(
                model="claude-3-opus-20240229",
                max_tokens=4000,
                temperature=0,
                system="""You are an expert Rust programmer tasked with analyzing code quality and suggesting fixes. 
                Focus on correctness, performance, security, and maintainability.
                
                IMPORTANT FIX FORMATTING GUIDELINES:
                1. Suggest small, targeted fixes rather than large rewrites
                2. Each fix should address a single issue
                3. Maintain the overall structure and organization of the original code
                4. Preserve function boundaries and indentation
                5. Ensure each fix is syntactically valid Rust code
                6. Prefer minimal changes that solve the issue
                7. Never combine multiple lines of code into a single line
                8. Never remove important newlines between functions or code blocks
                9. Ensure that all braces {}, parentheses (), and brackets [] are properly matched
                10. If you're unsure about a fix, suggest a safer, more conservative change

                These guidelines are crucial to ensure that your suggestions can be automatically applied without breaking the code structure.""",
                messages=[
                    {"role": "user", "content": prompt}
                ]
            )
            return response.content[0].text
        except Exception as e:
            print(f"Attempt {attempt+1}/{MAX_RETRIES} failed: {e}")
            if attempt < MAX_RETRIES - 1:
                time.sleep(RETRY_DELAY)
            else:
                raise

def main():
    # Get API key from environment
    api_key = os.environ.get('CLAUDE_API_KEY')
    if not api_key:
        print("Error: CLAUDE_API_KEY environment variable not set")
        sys.exit(1)
    
    # Read the analysis data
    clippy_output = read_file_if_exists('clippy_output.json')
    audit_output = read_file_if_exists('audit_output.json')
    project_info = read_file_if_exists('project_info.txt')
    
    # Get changed files for context
    changed_files = get_changed_files()
    file_samples = get_file_samples(changed_files)
    
    # Format the data for readability
    formatted_clippy = format_clippy_output(clippy_output)
    formatted_audit = format_audit_output(audit_output)
    
    # Read Cargo.toml for context
    cargo_toml = read_file_if_exists('Cargo.toml')
    
    # Construct the prompt for Claude
    prompt = f"""
# Rust Code Analysis Request

I need you to analyze the output from Rust code analysis tools and provide detailed feedback and fixes.

## Project Information
{project_info}

## Cargo.toml
```toml
{cargo_toml}
```

## Clippy Output
{formatted_clippy}

## Security Audit
{formatted_audit}

## Sample Code Files
{file_samples}

## Instructions

Please analyze the code based on the Clippy warnings, security audit, and file samples provided. Then:

1. Identify the most important issues to fix, prioritizing:
   - Security vulnerabilities
   - Potential bugs and logic errors
   - Performance issues
   - Code style and best practices

2. Provide a summary of the key issues found.

3. For each issue you identify, provide:
   - A clear explanation of the problem
   - The severity level (Critical, High, Medium, Low)
   - Specific code fixes using this format:

```
<FIXES>
file: path/to/file.rs
---
original: |
  // Original problematic code here (exact match required)
---
fixed: |
  // Fixed code here
---
explanation: |
  Explanation of what was fixed and why
</FIXES>
```

4. IMPORTANT REQUIREMENTS FOR FIXES:
   - Make each fix as small and focused as possible
   - Maintain the exact same code structure, including line breaks and indentation
   - Never combine multiple statements into a single line
   - Ensure each fix is syntactically valid Rust code
   - Do NOT remove newlines between function declarations or code blocks
   - Ensure all braces {{}}, parentheses (()), and brackets [[]] are properly paired
   - If a line contains complex code, it's better to leave it as-is than risk breaking it
   - Each fix should address a single issue only

5. Use the exact <FIXES> format for automatic code updates. Ensure the original code section exactly matches what's in the file.

6. Provide additional recommendations for improving the codebase that weren't directly flagged by the tools.

Focus on practical, high-value improvements that will make the code more reliable, secure, and maintainable.
"""
    
    # Call Claude API
    print("Calling Claude API for code analysis...")
    try:
        claude_response = call_claude_api(prompt, api_key)
        
        # Save the full response
        with open('claude_response.json', 'w', encoding='utf-8') as f:
            json.dump({'response': claude_response}, f, indent=2)
        
        # Extract the fixes section for the apply_fixes.py script
        fixes = []
        current_fix = None
        in_fixes_block = False
        
        for line in claude_response.split('\n'):
            if line.strip() == '<FIXES>':
                in_fixes_block = True
                current_fix = []
            elif line.strip() == '</FIXES>':
                in_fixes_block = False
                if current_fix:
                    fixes.append('\n'.join(current_fix))
                current_fix = None
            elif in_fixes_block and current_fix is not None:
                current_fix.append(line)
        
        # Save the extracted fixes
        with open('claude_fixes.json', 'w', encoding='utf-8') as f:
            json.dump({'fixes': fixes}, f, indent=2)
        
        # Create a simplified markdown report for the PR comment
        report_content = claude_response
        
        # If we have a <FIXES> section, remove it from the report to avoid confusion
        # This report is for humans, the fixes will be applied automatically
        report_content = report_content.split('<FIXES>')[0]
        if '</FIXES>' in report_content:
            report_content = report_content.split('</FIXES>')[-1]
        
        with open('claude_analysis_report.md', 'w', encoding='utf-8') as f:
            f.write(report_content)
        
        print("Analysis completed successfully")
    except Exception as e:
        print(f"Error during Claude API analysis: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
