#!/usr/bin/env python3
import json
import os

def apply_fixes(fixes_file):
    with open(fixes_file, "r", encoding="utf-8") as f:
        fixes = json.load(f)

    for fix in fixes:
        file_path = fix.get("file")
        changes = fix.get("changes", [])

        if not file_path or not os.path.exists(file_path):
            continue

        with open(file_path, "r", encoding="utf-8") as f:
            content = f.read()

        for change in changes:
            content = content.replace(change["original"], change["fixed"])

        with open(file_path, "w", encoding="utf-8") as f:
            f.write(content)

if __name__ == "__main__":
    apply_fixes("fixes.json")
