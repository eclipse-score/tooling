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

# Usage: check_diagram_link.sh <source> <alias> <target> <rootpath>...
#
# Asserts that the diagram_links.json among the given files has an element
# link from `<alias>` in the diagram `<source>` to the diagram `<target>`.
# Diagrams are matched by file name.

source_name="$1"
alias="$2"
target_name="$3"
shift 3

links_json=""
for rel_path in "$@"; do
    candidate="${TEST_SRCDIR}/${TEST_WORKSPACE}/${rel_path}"
    if [[ "${candidate}" == */diagram_links.json && -f "${candidate}" ]]; then
        links_json="${candidate}"
    fi
done

if [[ -z "${links_json}" ]]; then
    echo "Error: no diagram_links.json among: $*" >&2
    exit 1
fi

python3 - "${links_json}" "${source_name}" "${alias}" "${target_name}" <<'PY'
import json
import os
import sys

path, source, alias, target = sys.argv[1:]
with open(path) as f:
    links = json.load(f)["element_links"]
if not any(
    os.path.basename(link["source"]) == source
    and link["alias"] == alias
    and os.path.basename(link["target"]) == target
    for link in links
):
    print(f"Error: no element link {source}#{alias} -> {target} in:", file=sys.stderr)
    print(json.dumps(links, indent=1), file=sys.stderr)
    sys.exit(1)
PY

echo "ok"
