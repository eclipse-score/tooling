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
    name = "component_class_test_data",
    srcs = [
        "//validation/core/integration_test/component_class/negative_boundary_mismatch:case_data",
        "//validation/core/integration_test/component_class/negative_case_sensitive_mismatch:case_data",
        "//validation/core/integration_test/component_class/negative_missing_namespace_coverage:case_data",
        "//validation/core/integration_test/component_class/negative_multi_class_files_with_mismatch:case_data",
        "//validation/core/integration_test/component_class/positive_exact_match:case_data",
        "//validation/core/integration_test/component_class/positive_multi_class_files:case_data",
        "//validation/core/integration_test/component_class/positive_suffix_match:case_data",
    ],
)

rust_test(
    name = "component_class_integration_test",
    srcs = ["component_class_suite.rs"],
    crate_root = "component_class_suite.rs",
    data = [
        ":component_class_test_data",
    ],
    deps = [
        "//tools/metamodel/class:class_diagram",
        "//validation/core:validation_cli",
        "//validation/core/integration_test:test_framework",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
