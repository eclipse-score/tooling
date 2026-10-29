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
    name = "sequence_design_test_data",
    srcs = [
        "//validation/core/integration_test/sequence_design/negative_actor_call_other_units_interface:case_data",
        "//validation/core/integration_test/sequence_design/negative_actor_call_undeclared_method:case_data",
        "//validation/core/integration_test/sequence_design/negative_actor_names_unit:case_data",
        "//validation/core/integration_test/sequence_design/negative_callback_not_in_public_api:case_data",
        "//validation/core/integration_test/sequence_design/negative_conflicting_environment_declarations:case_data",
        "//validation/core/integration_test/sequence_design/negative_container_call_does_not_cover_unit_pair:case_data",
        "//validation/core/integration_test/sequence_design/negative_gate_message_undeclared_method:case_data",
        "//validation/core/integration_test/sequence_design/negative_legacy_external_endpoint:case_data",
        "//validation/core/integration_test/sequence_design/positive_actor_boundary:case_data",
        "//validation/core/integration_test/sequence_design/positive_container_lifeline:case_data",
        "//validation/core/integration_test/sequence_design/positive_create_message:case_data",
        "//validation/core/integration_test/sequence_design/positive_create_statement:case_data",
        "//validation/core/integration_test/sequence_design/positive_external_participant:case_data",
        "//validation/core/integration_test/sequence_design/positive_gate_message_declared_method:case_data",
    ],
)

rust_test(
    name = "sequence_design_integration_test",
    srcs = ["sequence_design_suite.rs"],
    crate_root = "sequence_design_suite.rs",
    data = [
        ":sequence_design_test_data",
    ],
    deps = [
        "//validation/core:validation_cli",
        "//validation/core/integration_test:test_framework",
        "@crates_score_tooling//:serde",
        "@crates_score_tooling//:serde_json",
    ],
)
