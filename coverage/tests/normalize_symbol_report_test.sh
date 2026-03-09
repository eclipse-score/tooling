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

fixture="${TEST_SRCDIR}/${TEST_WORKSPACE}/coverage/tests/fixtures/symbol_report.json"
script="${TEST_SRCDIR}/${TEST_WORKSPACE}/coverage/scripts/normalize_symbol_report.py"

workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

cp "${fixture}" "${workdir}/symbol_report.json"

python3 "${script}" "${workdir}/symbol_report.json" "/workspace" "/execroot"

python3 - "${workdir}/symbol_report.json" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)

files = sorted({s.get("filename") for s in data.get("symbols", [])})
expected = ["src/lib.rs", "src/rel.rs"]
if files != expected:
    raise SystemExit(f"unexpected filenames: {files}")

print("ok")
PY
