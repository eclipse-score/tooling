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

# Usage: check_unit_design_render.sh <html dir rootpath> <unit>:<marker>...
#
# Asserts, for every unit, that units/<unit>/index.html exists, that its
# design fragments are not rendered as standalone pages, and that the diagram
# on the page is the unit's own (its SVG contains <marker>).

html_dir="${TEST_SRCDIR}/${TEST_WORKSPACE}/$1"
shift

fail() {
    echo "Error: $*" >&2
    exit 1
}

for spec in "$@"; do
    unit="${spec%%:*}"
    marker="${spec#*:}"
    page="${html_dir}/units/${unit}/index.html"

    [[ -f "${page}" ]] || fail "missing ${page}"
    [[ ! -e "${html_dir}/units/${unit}/design" ]] ||
        fail "design fragments of ${unit} were rendered as standalone pages"

    svg_ref="$(grep -o '_images/plantuml-[0-9a-f]*\.svg' "${page}" | head -1 || true)"
    [[ -n "${svg_ref}" ]] || fail "${page} references no rendered diagram"
    grep -q "${marker}" "${html_dir}/${svg_ref}" ||
        fail "diagram on ${unit}'s page does not contain '${marker}'"
done

echo "✓ every unit page renders its own diagram"
