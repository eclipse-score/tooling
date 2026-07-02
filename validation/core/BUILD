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
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")

exports_files(
    [
        "README.md",
    ],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "specifications",
    srcs = [
        "docs/specifications/bazel_component.md",
        "docs/specifications/component_internal_api.md",
        "docs/specifications/component_sequence.md",
        "docs/specifications/sequence_internal_api.md",
    ],
    visibility = ["//visibility:public"],
)

rust_library(
    name = "validation",
    srcs = [
        "src/lib.rs",
        "src/models/bazel_models.rs",
        "src/models/class_diagram_models.rs",
        "src/models/component_diagram_models.rs",
        "src/models/mod.rs",
        "src/models/sequence_diagram_models.rs",
        "src/models/shared.rs",
        "src/profiles/architectural_design.rs",
        "src/profiles/dependable_element.rs",
        "src/profiles/mod.rs",
        "src/profiles/profile.rs",
        "src/readers/bazel_reader.rs",
        "src/readers/class_diagram_reader.rs",
        "src/readers/component_diagram_reader.rs",
        "src/readers/mod.rs",
        "src/readers/sequence_diagram_reader.rs",
        "src/results/diagnostics.rs",
        "src/results/mod.rs",
        "src/validators/bazel_component_validator.rs",
        "src/validators/component_internal_api_validator.rs",
        "src/validators/component_sequence_validator.rs",
        "src/validators/mod.rs",
        "src/validators/sequence_internal_api_validator.rs",
        "src/validators/shared/diagram_analysis.rs",
        "src/validators/shared/helpers.rs",
        "src/validators/shared/mod.rs",
        "src/validators/test/component_internal_api_validator_test.rs",
        "src/validators/test/component_sequence_validator_test.rs",
        "src/validators/test/fixtures.rs",
        "src/validators/test/sequence_internal_api_validator_test.rs",
    ],
    crate_root = "src/lib.rs",
    visibility = ["//visibility:public"],
    deps = [
        "//tools/metamodel/class:class_diagram",
        "//tools/metamodel/component:component_diagram",
        "//tools/metamodel/sequence:sequence_diagram",
        "//tools/serialization/flatbuffers/class:class_fbs",
        "//tools/serialization/flatbuffers/component:component_fbs",
        "//tools/serialization/flatbuffers/sequence:sequence_fbs",
        "@crates//:clap",
        "@crates//:flatbuffers",
        "@crates//:log",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

rust_binary(
    name = "validation_cli",
    srcs = [
        "src/main.rs",
        "src/report.rs",
    ],
    crate_root = "src/main.rs",
    visibility = ["//visibility:public"],
    deps = [
        ":validation",
        "@crates//:clap",
        "@crates//:env_logger",
        "@crates//:log",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

rust_test(
    name = "validation_test",
    crate = ":validation",
)
