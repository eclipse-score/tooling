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
    name = "component_sequence_test_data",
    srcs = [
        "//validation/core/integration_test/component_sequence/negative_component_unit_missing_from_sequence:case_data",
        "//validation/core/integration_test/component_sequence/negative_sequence_call_without_shared_interface:case_data",
        "//validation/core/integration_test/component_sequence/negative_shared_interface_without_sequence_call:case_data",
        "//validation/core/integration_test/component_sequence/positive_external_callee_return_allowed:case_data",
        "//validation/core/integration_test/component_sequence/positive_external_caller_call_allowed:case_data",
        "//validation/core/integration_test/component_sequence/positive_matching_component_and_sequence:case_data",
        "//validation/core/integration_test/component_sequence/positive_overview_preserves_unit_bindings:case_data",
    ],
)

rust_test(
    name = "component_sequence_integration_test",
    srcs = ["component_sequence_suite.rs"],
    crate_root = "component_sequence_suite.rs",
    data = [
        ":component_sequence_test_data",
    ],
    deps = [
        "//validation/core:validation_cli",
        "//validation/core/integration_test:test_framework",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
