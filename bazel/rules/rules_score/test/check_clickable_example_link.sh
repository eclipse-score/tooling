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

# Regression test for the clickable_plantuml Sphinx extension, backed by real
# Bazel-built artifacts (not just README prose).
#
# fixtures/clickable_example/overview.puml references `Proxy`, and
# fixtures/clickable_example/proxy_detail.puml defines it. clickable_plantuml
# resolves such cross-diagram references purely from the `*.idmap.json`
# sidecars produced by puml_cli/puml_idmap during the real
# :clickable_example_lib_doc Sphinx build, so asserting on their content here
# directly exercises the same parser + role-detection pipeline documented in
# //plantuml/sphinx/clickable_plantuml/README.md - independent of whether the
# external `plantuml` renderer binary itself is runnable in this sandbox
# (a pre-existing, unrelated hermeticity limitation of this test module).

overview_idmap=""
proxy_detail_idmap=""
for rel_path in "$@"; do
    candidate="${TEST_SRCDIR}/${TEST_WORKSPACE}/${rel_path}"
    case "${candidate}" in
        */overview.idmap.json) overview_idmap="${candidate}" ;;
        */proxy_detail.idmap.json) proxy_detail_idmap="${candidate}" ;;
    esac
done

if [[ -z "${overview_idmap}" || ! -f "${overview_idmap}" ]]; then
    echo "Error: could not locate overview.idmap.json among: $*" >&2
    exit 1
fi
if [[ -z "${proxy_detail_idmap}" || ! -f "${proxy_detail_idmap}" ]]; then
    echo "Error: could not locate proxy_detail.idmap.json among: $*" >&2
    exit 1
fi

# overview.puml must *reference* Proxy (no local definition)...
if ! grep -q '"alias": "Proxy"' "${overview_idmap}"; then
    echo "Error: expected overview.idmap.json to reference 'Proxy':" >&2
    cat "${overview_idmap}" >&2
    exit 1
fi

# ...and proxy_detail.puml must *define* it, so clickable_plantuml has exactly
# one definer to resolve the reference to.
if ! python3 -c "
import json, sys
data = json.load(open('${proxy_detail_idmap}'))
sys.exit(0 if any(e['id'] == 'Proxy' for e in data.get('defines', [])) else 1)
"; then
    echo "Error: expected proxy_detail.idmap.json to define 'Proxy':" >&2
    cat "${proxy_detail_idmap}" >&2
    exit 1
fi

echo "ok"
