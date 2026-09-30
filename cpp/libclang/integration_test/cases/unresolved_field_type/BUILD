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
    name = "unresolved_field_type",
    srcs = ["registration_result.cpp"],
    tags = ["manual"],  # intentionally fails to compile: event_control.h is never checked in
    visibility = ["//cpp/libclang:__subpackages__"],
)

# Regression test: an unresolved #include (here, a header that was never
# checked in) must not silently make the field it declares look like a
# plain `int` (clang's error-recovery placeholder type). The field's type
# must instead be reported as written in the source.
cpp_parser_integration_test(
    name = "test_unresolved_field_type",
    allow_parse_errors = True,
    expected_output = ["expected.json"],
    target = ":unresolved_field_type",
)
