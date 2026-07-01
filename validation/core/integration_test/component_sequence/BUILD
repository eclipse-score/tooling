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
        "//validation/core/integration_test/component_sequence/negative_missing_interface_connection_for_sequence_connected_units:case_data",
        "//validation/core/integration_test/component_sequence/negative_missing_participant:case_data",
        "//validation/core/integration_test/component_sequence/negative_missing_sequence_interaction_for_interface_connected_units:case_data",
        "//validation/core/integration_test/component_sequence/negative_missing_unit_interface_relation:case_data",
        "//validation/core/integration_test/component_sequence/negative_mixed_mismatch:case_data",
        "//validation/core/integration_test/component_sequence/negative_orphan_participant:case_data",
        "//validation/core/integration_test/component_sequence/positive_exact_match:case_data",
    ],
)

rust_test(
    name = "component_sequence_integration_test",
    srcs = ["component_sequence_suite.rs"],
    crate_root = "component_sequence_suite.rs",
    data = [
        ":component_sequence_test_data",
        "//validation/core:validation_cli",
    ],
    deps = [
        "//validation/core:validation",
        "//validation/core/integration_test:test_framework",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
