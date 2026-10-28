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

# Two dependency modules named dup_lib, defined in different packages, must
# each be merged into their own <repo>/<package>/<name> directory of the
# consumer's site, keeping their own content.
# $1 is the consumer's merged HTML directory.
html_dir="$1"
base="$html_dir/_main/fixtures/same_name_modules"

for pkg in a b; do
    page="$base/pkg_$pkg/dup_lib/index.html"
    if [[ ! -f "$page" ]]; then
        echo "Error: Expected $page not found" >&2
        exit 1
    fi

    expected="Same Name Module ${pkg^^}"
    if ! grep -q "$expected" "$page"; then
        echo "Error: '$expected' not found in $page" >&2
        exit 1
    fi
done

echo "✓ Same-named modules from different packages are both present"
