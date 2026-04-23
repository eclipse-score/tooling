#!/bin/bash
# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************
set -euo pipefail

# Test if the html output contains unknown needs.
# TODO: pass the HTML dir via args instead of using a hardcoded relative path,
# e.g. args = ["$(rootpath :module_a_lib)"] in the sh_test and read as "$1/index.html".
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
