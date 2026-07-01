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

rust_test(
    name = "bazel_component_integration_test",
    srcs = ["bazel_component_suite.rs"],
    crate_root = "bazel_component_suite.rs",
    data = [
        ":bazel_component_test_data",
        "//validation/core:validation_cli",
    ],
    deps = [
        "//validation/core:validation",
        "//validation/core/integration_test:test_framework",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
