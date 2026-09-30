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
    name = "unresolved_param_and_return",
    srcs = ["registration_service.cpp"],
    tags = ["manual"],  # intentionally fails to compile: event_control.h is never checked in
    visibility = ["//cpp/libclang:__subpackages__"],
)

# Regression test: same guard as unresolved_field_type, but for
# a method's parameter and return type instead of a field.
cpp_parser_integration_test(
    name = "test_unresolved_param_and_return",
    allow_parse_errors = True,
    expected_output = ["expected.json"],
    target = ":unresolved_param_and_return",
)
