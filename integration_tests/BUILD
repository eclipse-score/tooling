# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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

# Integration tests for the unified score_tooling module

load("@score_tooling//format_checker:macros.bzl", "use_format_targets")

# Test that format checker macro works
use_format_targets()

# Test filegroup to demonstrate that tools are accessible
filegroup(
    name = "test_sources",
    srcs = [
        "test_file.py",
        "test_file.yaml",
        "BUILD",
    ],
)

# Test that we can load and use other tools  
sh_test(
    name = "test_unified_module_loads",
    srcs = ["test_unified_module.sh"],
    data = [
        "@score_tooling//cr_checker/tool:cr_checker_lib",
        "@score_tooling//dash:jar",
    ],
)