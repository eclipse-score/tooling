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

load("@rules_rust//rust:defs.bzl", "rust_test")

filegroup(
    name = "dependable_element_zoom_test_data",
    srcs = [
        "//validation/core/integration_test/dependable_element_zoom/negative_cross_design_exit_missing:case_data",
        "//validation/core/integration_test/dependable_element_zoom/positive_cross_design:case_data",
    ],
)

rust_test(
    name = "dependable_element_zoom_integration_test",
    srcs = ["dependable_element_zoom_suite.rs"],
    crate_root = "dependable_element_zoom_suite.rs",
    data = [
        ":dependable_element_zoom_test_data",
    ],
    deps = [
        "//validation/core:validation_cli",
        "//validation/core/integration_test:test_framework",
        "@crates_score_tooling//:serde_json",
    ],
)
