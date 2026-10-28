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

# Usage: check_validation_passes.sh <log suffix> <runfile rootpath>...
#
# The dependable element's architecture validation log must report PASS. Under
# maturity = "development" a failure is only a warning, so the build alone does
# not prove the architecture matches the diagram.

source "${TEST_SRCDIR}/${TEST_WORKSPACE}/lib/find_runfile.sh"

log_suffix="$1"
shift

log_file=$(find_runfile "${log_suffix}" "$@")

if [[ "$(cat "${log_file}")" != "PASS" ]]; then
    echo "Error: validation did not pass:" >&2
    cat "${log_file}" >&2
    exit 1
fi

echo "✓ architecture validation passed"
