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

load("@rules_shell//shell:sh_binary.bzl", "sh_binary")

package(default_visibility = ["//visibility:public"])

exports_files([
    "ferrocene_report_wrapper.sh.tpl",
    "scripts/normalize_symbol_report.py",
    "scripts/parse_line_coverage.py",
])

sh_binary(
    name = "ferrocene_report",
    srcs = ["ferrocene_report.sh"],
    data = [
        "scripts/normalize_symbol_report.py",
        "scripts/parse_line_coverage.py",
    ],
    visibility = ["//visibility:public"],
)

sh_binary(
    name = "llvm_profile_wrapper",
    srcs = ["llvm_profile_wrapper.sh"],
    visibility = ["//visibility:public"],
)

sh_binary(
    name = "combined_report",
    srcs = ["run_combined_coverage.sh"],
    data = ["@lcov_deb//:srcs"],
)
