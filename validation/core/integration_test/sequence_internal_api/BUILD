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
        "//validation/core/integration_test/sequence_internal_api/negative_interface_function_not_exercised:case_data",
        "//validation/core/integration_test/sequence_internal_api/negative_invalid_consumer_provider_direction:case_data",
        "//validation/core/integration_test/sequence_internal_api/negative_method_available_but_not_on_related_interface:case_data",
        "//validation/core/integration_test/sequence_internal_api/negative_missing_method_in_related_interface:case_data",
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
