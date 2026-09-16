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

# Regression test for the compose/override reconciliation, at the level of the
# actual *staged* RST content dependable_element produces for Sphinx to consume
# -- see plan_view_layout()'s docstring in puml_utils.bzl.
#
# $1 selects which scenario to check: "compose", "override", or "diagram_free".
# Remaining args are the `$(rootpaths :authored_layout_example_lib_index)`
# runfiles paths.

mode="$1"
shift

find_file() {
    local suffix="$1"
    shift
    for rel_path in "$@"; do
        candidate="${TEST_SRCDIR}/${TEST_WORKSPACE}/${rel_path}"
        if [[ -f "${candidate}" && "${candidate}" == *"${suffix}" ]]; then
            echo "${candidate}"
            return 0
        fi
    done
    echo "Error: could not locate '*${suffix}' among: $*" >&2
    return 1
}

case "${mode}" in
    compose)
        index_file=$(find_file "arch_design_authored_index_compose_repro/static/fixtures/authored/index.rst" "$@")
        inc_file=$(find_file "arch_design_authored_index_compose_repro/static/fixtures/authored/index.rst.inc" "$@")

        # The generated index.rst must include the authored body instead of
        # emitting its own title/marker.
        if ! grep -Fq '.. include:: index.rst.inc' "${index_file}"; then
            echo "Error: expected 'compose' index.rst to include the authored body:" >&2
            cat "${index_file}" >&2
            exit 1
        fi

        # The generated toctree must still list the auto-wrapped diagram.
        if ! grep -q '^   overview$' "${index_file}"; then
            echo "Error: expected 'compose' index.rst toctree to still list 'overview':" >&2
            cat "${index_file}" >&2
            exit 1
        fi

        # The staged .inc file must carry the authored title/prose verbatim.
        if ! grep -Fq 'Authored Overview' "${inc_file}"; then
            echo "Error: expected staged index.rst.inc to carry the authored title/prose:" >&2
            cat "${inc_file}" >&2
            exit 1
        fi
        ;;
    override)
        overview_file=$(find_file "arch_design_authored_rst_override_repro/static/fixtures/authored_override/overview.rst" "$@")

        # The staged overview.rst must be the authored file verbatim...
        if ! grep -Fq 'Hand-authored prose for the overview diagram' "${overview_file}"; then
            echo "Error: expected staged overview.rst to be the authored file, not a generated wrapper:" >&2
            cat "${overview_file}" >&2
            exit 1
        fi

        # ...not the generated ".. uml::" wrapper placeholder it suppresses.
        if grep -Fq '.. uml::' "${overview_file}"; then
            echo "Error: staged overview.rst still contains the generated '.. uml::' wrapper directive; authored override should have suppressed it:" >&2
            cat "${overview_file}" >&2
            exit 1
        fi
        ;;
    diagram_free)
        index_file=$(find_file "arch_design_diagram_free_repro/fixtures/diagram_free/index.rst" "$@")
        inc_file=$(find_file "arch_design_diagram_free_repro/fixtures/diagram_free/index.md.inc" "$@")

        # The generated index.rst must include the authored markdown body.
        if ! grep -Fq '.. include:: index.md.inc' "${index_file}"; then
            echo "Error: expected diagram-free index.rst to include the authored body:" >&2
            cat "${index_file}" >&2
            exit 1
        fi

        # There are no diagrams to auto-wrap, so the toctree must be empty
        # (no entries at all after the "maxdepth" line).
        if [[ $(grep -c '^   [^:[:space:]]' "${index_file}") -ne 0 ]]; then
            echo "Error: expected diagram-free index.rst toctree to have no entries:" >&2
            cat "${index_file}" >&2
            exit 1
        fi

        # The staged .inc file must carry the authored prose verbatim.
        if ! grep -Fq 'Diagram-Free Overview' "${inc_file}"; then
            echo "Error: expected staged index.md.inc to carry the authored prose:" >&2
            cat "${inc_file}" >&2
            exit 1
        fi
        ;;
    *)
        echo "Error: unknown mode '${mode}' (expected 'compose', 'override', or 'diagram_free')" >&2
        exit 1
        ;;
esac

echo "ok"
