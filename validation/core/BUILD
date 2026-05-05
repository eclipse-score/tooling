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

rust_library(
    name = "validation",
    srcs = [
        "src/lib.rs",
        "src/models/bazel_models.rs",
        "src/models/class_diagram_models.rs",
        "src/models/component_diagram_models.rs",
        "src/models/error_models.rs",
        "src/models/mod.rs",
        "src/models/shared.rs",
        "src/readers/bazel_reader.rs",
        "src/readers/class_diagram_reader.rs",
        "src/readers/component_diagram_reader.rs",
        "src/readers/mod.rs",
        "src/validators/bazel_component_validator.rs",
        "src/validators/component_class_validator.rs",
        "src/validators/component_sequence_validator.rs",
        "src/validators/mod.rs",
    ],
    crate_root = "src/lib.rs",
    visibility = ["//visibility:public"],
    deps = [
        "//plantuml/parser/puml_serializer/src/fbs:class_fbs",
        "//plantuml/parser/puml_serializer/src/fbs:component_fbs",
        "@crates//:flatbuffers",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

rust_binary(
    name = "validation_cli",
    srcs = ["src/main.rs"],
    crate_root = "src/main.rs",
    visibility = ["//visibility:public"],
    deps = [
        ":validation",
        "@crates//:clap",
        "@crates//:env_logger",
        "@crates//:log",
    ],
)

rust_test(
    name = "validation_test",
    crate = ":validation",
)
