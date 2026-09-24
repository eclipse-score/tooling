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
    name = "component_public_api_test_data",
    srcs = [
        "//validation/core/integration_test/component_public_api/negative_public_api_case_mismatch:case_data",
        "//validation/core/integration_test/component_public_api/negative_public_api_declared_as_class_is_treated_as_missing:case_data",
        "//validation/core/integration_test/component_public_api/negative_public_api_without_seooc_relationship:case_data",
        "//validation/core/integration_test/component_public_api/negative_public_apis_missing_from_public_api_diagram:case_data",
        "//validation/core/integration_test/component_public_api/negative_public_apis_missing_from_public_api_diagram_with_suggestions:case_data",
        "//validation/core/integration_test/component_public_api/positive_overview_seooc_relation_merge:case_data",
        "//validation/core/integration_test/component_public_api/positive_public_api_declared_and_related_to_seooc:case_data",
    ],
)

rust_test(
    name = "component_public_api_integration_test",
    srcs = ["component_public_api_suite.rs"],
    crate_root = "component_public_api_suite.rs",
    data = [
        ":component_public_api_test_data",
    ],
    deps = [
        "//validation/core:validation_cli",
        "//validation/core/integration_test:test_framework",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
