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

# Usage: check_unit_design_render.sh <html dir rootpath> <unit page dir>:<marker>...
#
# Asserts, for every unit page directory (relative to the HTML root, ending in
# units/<repo>/<package>/<name>), that index.html exists, that another page
# links to it (a unit missing from every toctree is orphaned), and that the
# diagram on the page is the unit's own (its SVG contains <marker>). Design
# fragments must not be rendered as standalone pages: index.html is the only
# page there.

html_dir="${TEST_SRCDIR}/${TEST_WORKSPACE}/$1"
shift

fail() {
    echo "Error: $*" >&2
    exit 1
}

for spec in "$@"; do
    unit_rel="${spec%%:*}"
    unit_dir="${html_dir}/${unit_rel}"
    marker="${spec#*:}"
    page="${unit_dir}/index.html"

    [[ -f "${page}" ]] || fail "missing ${page}"

    linking_pages="$(grep -RlF --include='*.html' "units/${unit_rel#*units/}/index.html" "${html_dir}" |
        grep -vxF "${page}" || true)"
    [[ -n "${linking_pages}" ]] || fail "no page links to ${page}"

    standalone="$(find -L "${unit_dir}" -name '*.html' ! -name index.html)"
    [[ -z "${standalone}" ]] ||
        fail "design fragments were rendered as standalone pages: ${standalone}"

    svg_ref="$(grep -o '_images/plantuml-[0-9a-f]*\.svg' "${page}" | head -1 || true)"
    [[ -n "${svg_ref}" ]] || fail "${page} references no rendered diagram"
    grep -q "${marker}" "${html_dir}/${svg_ref}" ||
        fail "diagram on ${page} does not contain '${marker}'"
done

echo "✓ every unit page renders its own diagram"
