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
    name = "class_design_sequence_test_data",
    srcs = [
        "//validation/core/integration_test/class_design_sequence/negative_participant_ambiguous_short_name:case_data",
        "//validation/core/integration_test/class_design_sequence/negative_participant_method_missing:case_data",
        "//validation/core/integration_test/class_design_sequence/negative_participant_missing:case_data",
        "//validation/core/integration_test/class_design_sequence/negative_participant_private_inherited_method:case_data",
        "//validation/core/integration_test/class_design_sequence/positive_participant_abstract_base_method_match:case_data",
        "//validation/core/integration_test/class_design_sequence/positive_participant_alias_display_name_class_name_match:case_data",
        "//validation/core/integration_test/class_design_sequence/positive_participant_alias_display_name_namespace_match:case_data",
        "//validation/core/integration_test/class_design_sequence/positive_participant_multilevel_inherited_method_match:case_data",
        "//validation/core/integration_test/class_design_sequence/positive_participant_namespace_callee_method_match:case_data",
        "//validation/core/integration_test/class_design_sequence/positive_participant_short_name_namespace_match:case_data",
    ],
)

rust_test(
    name = "class_design_sequence_integration_test",
    srcs = ["class_design_sequence_suite.rs"],
    crate_root = "class_design_sequence_suite.rs",
    data = [
        ":class_design_sequence_test_data",
    ],
    deps = [
        "//validation/core:validation_cli",
        "//validation/core/integration_test:test_framework",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
