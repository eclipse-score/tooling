#!/bin/bash
set -euo pipefail

# Test if the html output contains unknown needs
html_file="./module_a_lib/html/index.html"

if [[ ! -f "$html_file" ]]; then
    echo "Error: File not found: $html_file" >&2
    exit 1
fi

if grep -q "Unknown need" "$html_file"; then
    echo "Error: Found 'Unknown need' in $html_file" >&2
    exit 1
fi

echo "✓ No unknown needs found"
