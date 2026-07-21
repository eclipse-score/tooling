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

# Generic regression-test helper for clickable_plantuml example fixtures.
#
# Usage: check_idmap_link.sh <expected_id> <idmap.json rootpath>...
#
# Asserts that, among the given `*.idmap.json` files (real Bazel-built
# artifacts produced by puml_cli/puml_idmap), `<expected_id>` appears in at
# least one file's "defines" list AND in at least one file's "references"
# list - i.e. that clickable_plantuml has exactly what it needs (a reference
# plus a matching definer) to make that element clickable, regardless of
# which diagram type (component, class, interface, ...) produced each file.

expected_id="$1"
shift

idmap_paths=()
for rel_path in "$@"; do
    candidate="${TEST_SRCDIR}/${TEST_WORKSPACE}/${rel_path}"
    if [[ -f "${candidate}" && "${candidate}" == *.idmap.json ]]; then
        idmap_paths+=("${candidate}")
    fi
done

if [[ "${#idmap_paths[@]}" -eq 0 ]]; then
    echo "Error: no *.idmap.json files found among: $*" >&2
    exit 1
fi

python3 -c "
import json, sys

expected_id = sys.argv[1]
paths = sys.argv[2:]

has_define = False
has_reference = False
for p in paths:
    with open(p) as f:
        data = json.load(f)
    if any(e['id'] == expected_id for e in data.get('defines', [])):
        has_define = True
    if any(e['id'] == expected_id for e in data.get('references', [])):
        has_reference = True

if not has_define:
    print(f'Error: no idmap.json defines id {expected_id!r}', file=sys.stderr)
    sys.exit(1)
if not has_reference:
    print(f'Error: no idmap.json references id {expected_id!r}', file=sys.stderr)
    sys.exit(1)
print('ok')
" "${expected_id}" "${idmap_paths[@]}"
