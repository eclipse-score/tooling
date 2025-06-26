#!/bin/bash

# Test script to validate that the unified module works correctly

set -euo pipefail

echo "Testing unified score_tooling module..."

# Check that cr_checker library is available
if [[ -f "$1" ]] || [[ -d "$1" ]]; then
    echo "✓ cr_checker library found"
else
    echo "✗ cr_checker library not found at $1"
    exit 1
fi

# Check that dash jar is available
if [[ -f "$2" ]]; then
    echo "✓ dash jar found"
else
    echo "✗ dash jar not found at $2"
    exit 1
fi

echo "✓ All tools accessible from unified module"