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
    name = "type_alias",
    srcs = glob(["*.cpp"]),
    visibility = ["//cpp/libclang:__subpackages__"],
)

cpp_parser_integration_test(
    name = "test_type_alias",
    expected_output = ["expected.json"],
    target = ":type_alias",
)
