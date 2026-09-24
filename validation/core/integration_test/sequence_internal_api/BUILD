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
    name = "sequence_internal_api_test_data",
    srcs = [
        "//validation/core/integration_test/sequence_internal_api/negative_internal_api_method_not_exercised:case_data",
        "//validation/core/integration_test/sequence_internal_api/negative_method_declared_on_unrelated_interface:case_data",
        "//validation/core/integration_test/sequence_internal_api/negative_method_missing_with_suggestion:case_data",
        "//validation/core/integration_test/sequence_internal_api/negative_method_not_declared_in_related_interface:case_data",
        "//validation/core/integration_test/sequence_internal_api/negative_sequence_call_direction_mismatch:case_data",
        "//validation/core/integration_test/sequence_internal_api/positive_internal_api_method_match:case_data",
        "//validation/core/integration_test/sequence_internal_api/positive_self_call_method_match:case_data",
    ],
)

rust_test(
    name = "sequence_internal_api_integration_test",
    srcs = ["sequence_internal_api_suite.rs"],
    crate_root = "sequence_internal_api_suite.rs",
    data = [
        ":sequence_internal_api_test_data",
    ],
    deps = [
        "//validation/core:validation_cli",
        "//validation/core/integration_test:test_framework",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
