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

# $1 is the expected index.rst path (suffix match against runfiles paths),
# e.g. "seooc_test_lib_index/index.rst" -- required because dependable_element
# generates many index.rst files (one per architectural_design view/directory
# plus its own top-level index), so a bare "*/index.rst" suffix match is
# ambiguous.

source "${TEST_SRCDIR}/${TEST_WORKSPACE}/lib/find_runfile.sh"

expected_suffix="$1"
shift

index_file=$(find_runfile "${expected_suffix}" "$@")

if ! grep -Fq '* `Dep Seooc Lib <dep_seooc_lib_doc/index.html>`_' "${index_file}"; then
    echo "Error: expected submodule link to dep_seooc_lib_doc/index.html in ${index_file}" >&2
    exit 1
fi

echo "ok"
