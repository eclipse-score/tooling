#!/bin/bash
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

# End-to-end regression test for the diamond-dependency HTML-merge
# flattening: module_d_lib is a dep of both module_b_lib and module_c_lib
# (never directly of module_a_lib), so before the fix its HTML was copied
# once per path that reached it -- nested under module_b_lib/module_d_lib/
# AND module_c_lib/module_d_lib/, never at the top level. After the fix it
# must land exactly once, flat, at the top level of the merged site.
# TODO: pass the HTML dir via args instead of using a hardcoded relative path,
# e.g. args = ["$(rootpath :module_a_lib)"] in the sh_test.
html_dir="./module_a_lib/html"

if [[ ! -d "$html_dir" ]]; then
    echo "Error: Directory not found: $html_dir" >&2
    exit 1
fi

if [[ ! -f "$html_dir/module_d_lib/index.html" ]]; then
    echo "Error: Expected flat, top-level $html_dir/module_d_lib/index.html not found" >&2
    exit 1
fi

if [[ -d "$html_dir/module_b_lib/module_d_lib" ]]; then
    echo "Error: module_d_lib was duplicated, nested under module_b_lib/" >&2
    exit 1
fi

if [[ -d "$html_dir/module_c_lib/module_d_lib" ]]; then
    echo "Error: module_d_lib was duplicated, nested under module_c_lib/" >&2
    exit 1
fi

echo "✓ module_d_lib's HTML appears exactly once, flat, in the merged output"
