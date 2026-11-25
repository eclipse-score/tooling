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

# Usage: check_no_zoom_findings.sh <validation log>
#
# The validation log must not report zoom (sequence refinement) findings.
# Other findings are ignored: the fixture is not a complete dependable element.

log_file="$1"

if grep -q "Zoom" "${log_file}"; then
    echo "Error: zoom findings in the validation log:" >&2
    cat "${log_file}" >&2
    exit 1
fi

echo "✓ no zoom findings"
