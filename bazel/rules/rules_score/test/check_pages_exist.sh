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

# Usage: check_pages_exist.sh <html dir rootpath> <page relative to html dir>...
html_dir="${TEST_SRCDIR}/${TEST_WORKSPACE}/$1"
shift

for page in "$@"; do
    [[ -f "${html_dir}/${page}" ]] || {
        echo "Error: missing ${html_dir}/${page}" >&2
        exit 1
    }
done

echo "✓ all expected pages rendered"
