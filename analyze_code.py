#!/usr/bin/env python3
import os
import json
import requests

API_KEY = os.getenv("CLAUDE_API_KEY")
API_URL = "https://api.anthropic.com/v1/messages"

def read_file(filename):
    if os.path.exists(filename):
        with open(filename, "r", encoding="utf-8") as f:
            return f.read()
    return ""

def send_request_to_claude():
    clippy_results = read_file("clippy-results.json")
    audit_results = read_file("audit-results.json")

    payload = {
        "model": "claude-3-7-sonnet-20250219",
        "max_tokens": 4000,
        "temperature": 0.2,
        "messages": [
            {
                "role": "user",
                "content": f"Analyze this Rust project and provide specific fixes.\n\nClippy Warnings:\n{clippy_results}\n\nAudit Results:\n{audit_results}"
            }
        ]
    }

    headers = {
        "x-api-key": API_KEY,
        "anthropic-version": "2023-06-01",
        "content-type": "application/json"
    }

    response = requests.post(API_URL, headers=headers, json=payload)
    if response.status_code == 200:
        with open("claude-analysis-report.md", "w", encoding="utf-8") as f:
            f.write(response.json().get("content", [{}])[0].get("text", "No response"))

        print("fixes_available=true")
    else:
        print("fixes_available=false")

if __name__ == "__main__":
    send_request_to_claude()
