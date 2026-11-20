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

exports_files(["api_surface.bzl"])

py_library(
    name = "diff_api_lib",
    srcs = ["diff_api.py"],
    visibility = ["//api_surface:__subpackages__"],
)

# The binaries are executed by the api_surface rules, possibly from other repositories.
py_binary(
    name = "extract_api",
    srcs = ["extract_api.py"],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "diff_api",
    srcs = ["diff_api.py"],
    visibility = ["//visibility:public"],
)
