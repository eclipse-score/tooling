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

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

filegroup(
    name = "bazel_component_test_data",
    srcs = [
        "//validation/core/integration_test/bazel_component/negative_extra_component:case_data",
        "//validation/core/integration_test/bazel_component/negative_extra_unit:case_data",
        "//validation/core/integration_test/bazel_component/negative_missing_component:case_data",
        "//validation/core/integration_test/bazel_component/negative_missing_unit:case_data",
        "//validation/core/integration_test/bazel_component/negative_wrong_stereotype:case_data",
        "//validation/core/integration_test/bazel_component/positive_case_insensitive:case_data",
        "//validation/core/integration_test/bazel_component/positive_component:case_data",
    ],
)

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

filegroup(
    name = "component_sequence_test_data",
    srcs = [
        "//validation/core/integration_test/component_sequence/negative_interface_function_not_exercised:case_data",
        "//validation/core/integration_test/component_sequence/negative_invalid_consumer_provider_direction:case_data",
        "//validation/core/integration_test/component_sequence/negative_invalid_self_call_consumer_provider_roles:case_data",
        "//validation/core/integration_test/component_sequence/negative_method_missing_from_internal_api:case_data",
        "//validation/core/integration_test/component_sequence/negative_missing_interface_connection_for_sequence_connected_units:case_data",
        "//validation/core/integration_test/component_sequence/negative_missing_method_in_related_interface:case_data",
        "//validation/core/integration_test/component_sequence/negative_missing_participant:case_data",
        "//validation/core/integration_test/component_sequence/negative_missing_sequence_interaction_for_interface_connected_units:case_data",
        "//validation/core/integration_test/component_sequence/negative_missing_unit_interface_relation:case_data",
        "//validation/core/integration_test/component_sequence/negative_mixed_mismatch:case_data",
        "//validation/core/integration_test/component_sequence/negative_orphan_participant:case_data",
        "//validation/core/integration_test/component_sequence/positive_exact_match:case_data",
        "//validation/core/integration_test/component_sequence/positive_internal_api_method_match:case_data",
        "//validation/core/integration_test/component_sequence/positive_self_call_method_match:case_data",
    ],
)

rust_library(
    name = "test_framework",
    srcs = [
        "src/lib.rs",
        "src/test_framework.rs",
    ],
    crate_root = "src/lib.rs",
    visibility = ["//validation/core:__subpackages__"],
    deps = [
        "//validation/core:validation",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

rust_test(
    name = "bazel_component_integration_test",
    srcs = ["src/bazel_component_suite.rs"],
    crate_root = "src/bazel_component_suite.rs",
    data = [
        ":bazel_component_test_data",
        "//validation/core:validation_cli",
    ],
    deps = [
        ":test_framework",
        "//validation/core:validation",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

rust_test(
    name = "component_class_integration_test",
    srcs = ["src/component_class_suite.rs"],
    crate_root = "src/component_class_suite.rs",
    data = [
        ":component_class_test_data",
        "//validation/core:validation_cli",
    ],
    deps = [
        ":test_framework",
        "//tools/metamodel/class:class_diagram",
        "//validation/core:validation",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

rust_test(
    name = "component_sequence_integration_test",
    srcs = ["src/component_sequence_suite.rs"],
    crate_root = "src/component_sequence_suite.rs",
    data = [
        ":component_sequence_test_data",
        "//validation/core:validation_cli",
    ],
    deps = [
        ":test_framework",
        "//validation/core:validation",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
