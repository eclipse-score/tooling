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

fixture="${TEST_SRCDIR}/${TEST_WORKSPACE}/coverage/tests/fixtures/blanket_index.html"
parser="${TEST_SRCDIR}/${TEST_WORKSPACE}/coverage/scripts/parse_line_coverage.py"

output="$(python3 "${parser}" "${fixture}")"

if [[ "${output}" != "100.00 8 8" ]]; then
  echo "unexpected coverage summary: ${output}" >&2
  exit 1
fi

echo "ok"
