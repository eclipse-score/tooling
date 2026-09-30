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
load("//cpp/libclang/integration_test:test_rules.bzl", "cpp_parser_integration_test")

cc_library(
    name = "event_control",
    hdrs = ["event_control.h"],
    visibility = ["//cpp/libclang:__subpackages__"],
)

cc_library(
    name = "implementation_deps_header",
    srcs = ["registration_result.cpp"],
    implementation_deps = [
        ":event_control",
    ],
    visibility = ["//cpp/libclang:__subpackages__"],
)

cpp_parser_integration_test(
    name = "test_implementation_deps_header",
    expected_output = ["expected.json"],
    target = ":implementation_deps_header",
)
