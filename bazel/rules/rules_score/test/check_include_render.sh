#!/usr/bin/env bash
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

# Usage: check_include_render.sh <html dir rootpath> <marker>...
#
# Asserts that one rendered diagram contains every <marker>, i.e. that
# PlantUML resolved the `!include` of the diagram and drew the included lines
# next to its own.

html_dir="${TEST_SRCDIR}/${TEST_WORKSPACE}/$1"
shift

fail() {
    echo "Error: $*" >&2
    exit 1
}

mapfile -t svgs < <(find -L "${html_dir}" -name 'plantuml-*.svg')
[[ "${#svgs[@]}" -gt 0 ]] || fail "no rendered diagram under ${html_dir}"

for svg in "${svgs[@]}"; do
    missing=0
    for marker in "$@"; do
        grep -q "${marker}" "${svg}" || missing=1
    done
    if [[ "${missing}" -eq 0 ]]; then
        echo "✓ ${svg##*/} contains: $*"
        exit 0
    fi
done

fail "no rendered diagram contains all of: $*"
