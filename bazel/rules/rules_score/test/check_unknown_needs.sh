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
# TODO: pass the HTML dirs via args instead of using hardcoded relative paths,
# e.g. args = ["$(rootpath :module_a_lib)", "$(rootpath :module_e_lib)"] in the
# sh_test and read as "$1/index.html", "$2/index.html".
#
# module_a_lib covers direct-dep needs resolution; module_e_lib covers the
# two-hop case (its :need: reference resolves only through module_b_lib ->
# module_d_lib, with no direct edge of its own to the defining module) --
# see needs_modules in sphinx_module.bzl for the mechanism this exercises.
html_files=(
    "./module_a_lib/html/index.html"
    "./module_e_lib/html/index.html"
)

for html_file in "${html_files[@]}"; do
    if [[ ! -f "$html_file" ]]; then
        echo "Error: File not found: $html_file" >&2
        exit 1
    fi

    if grep -q "Unknown need" "$html_file"; then
        echo "Error: Found 'Unknown need' in $html_file" >&2
        exit 1
    fi
done

echo "✓ No unknown needs found"
