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

index_file=""
for rel_path in "$@"; do
    candidate="${TEST_SRCDIR}/${TEST_WORKSPACE}/${rel_path}"
    if [[ -f "${candidate}" && "${candidate}" == */index.rst ]]; then
        index_file="${candidate}"
        break
    fi
done

if [[ -z "${index_file}" ]]; then
    echo "Error: Could not locate index.rst in provided runfiles paths: $*" >&2
    exit 1
fi

if ! grep -Fq '* `Dep Seooc Lib <dep_seooc_lib_doc/index.html>`_' "${index_file}"; then
    echo "Error: expected submodule link to dep_seooc_lib_doc/index.html in ${index_file}" >&2
    exit 1
fi

echo "ok"
