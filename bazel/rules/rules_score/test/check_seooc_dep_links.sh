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

# $1 is the dependable element's top-level index.rst.

index_file="$1"

if ! grep -Fq '* `//:dep_seooc_lib <_main/dep_seooc_lib_doc/index.html>`_' "${index_file}"; then
    echo "Error: expected submodule link to _main/dep_seooc_lib_doc/index.html in ${index_file}" >&2
    exit 1
fi

echo "ok"
