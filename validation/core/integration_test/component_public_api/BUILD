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
        "//validation/core/integration_test/component_public_api/negative_case_sensitive:case_data",
        "//validation/core/integration_test/component_public_api/negative_public_api_lack_of_relationship:case_data",
        "//validation/core/integration_test/component_public_api/negative_public_api_missing:case_data",
        "//validation/core/integration_test/component_public_api/negative_public_api_wrong_type:case_data",
        "//validation/core/integration_test/component_public_api/positive_public_api_match:case_data",
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
