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
    name = "architectural_unit_class_design_test_data",
    srcs = [
        "//validation/core/integration_test/architectural_unit_class_design/negative_missing_class:case_data",
        "//validation/core/integration_test/architectural_unit_class_design/negative_path_prefix:case_data",
        "//validation/core/integration_test/architectural_unit_class_design/positive_refining_class:case_data",
    ],
)

rust_test(
    name = "architectural_unit_class_design_integration_test",
    srcs = ["architectural_unit_class_design_suite.rs"],
    crate_root = "architectural_unit_class_design_suite.rs",
    data = [":architectural_unit_class_design_test_data"],
    deps = [
        "//validation/core:validation_cli",
        "//validation/core/integration_test:test_framework",
        "@crates//:serde_json",
    ],
)
