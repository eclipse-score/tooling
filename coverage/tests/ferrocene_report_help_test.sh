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

script="${TEST_SRCDIR}/${TEST_WORKSPACE}/coverage/ferrocene_report.sh"

output="$(bash "${script}" --help)"

if ! echo "${output}" | grep -q "Generate Ferrocene Rust coverage reports"; then
  echo "help output did not contain expected header" >&2
  exit 1
fi

if ! echo "${output}" | grep -q -- "--min-line-coverage"; then
  echo "help output missing --min-line-coverage" >&2
  exit 1
fi

echo "ok"
