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

load("@rules_python//python:defs.bzl", "py_binary", "py_library")

py_library(
    name = "lobster_rst_report_lib",
    srcs = [
        "__init__.py",
        "_helpers.py",
        "_renderers.py",
        "graphviz_utils.py",
        "rst_report.py",
    ],
    imports = [".."],
    deps = ["@lobster//lobster/common"],
)

py_binary(
    name = "lobster-rst-report",
    srcs = ["lobster-rst-report.py"],
    visibility = ["//visibility:public"],
    deps = [":lobster_rst_report_lib"],
)
