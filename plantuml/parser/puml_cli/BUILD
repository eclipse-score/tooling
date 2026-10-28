# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_test")

_SRCS = [
    "src/fta.rs",
    "src/inputs.rs",
    "src/main.rs",
    "src/parsing.rs",
    "src/resolving.rs",
    "src/sinks.rs",
]

rust_binary(
    name = "puml_cli",
    srcs = _SRCS,
    crate_root = "src/main.rs",
    visibility = ["//visibility:public"],
    deps = [
        "//plantuml/parser/puml_api",
        "//plantuml/parser/puml_fta",
        "//plantuml/parser/puml_idmap",
        "//plantuml/parser/puml_parser",
        "//plantuml/parser/puml_resolver",
        "//plantuml/parser/puml_utils",
        "//tools/metamodel/activity:activity_diagram",
        "//tools/metamodel/class:class_diagram",
        "//tools/metamodel/component:component_diagram",
        "//tools/metamodel/sequence:sequence_diagram",
        "//tools/serialization/flatbuffers/activity:activity_serializer",
        "//tools/serialization/flatbuffers/class:class_serializer",
        "//tools/serialization/flatbuffers/component:component_serializer",
        "//tools/serialization/flatbuffers/sequence:sequence_serializer",
        "@crates_score_tooling//:clap",
        "@crates_score_tooling//:env_logger",
        "@crates_score_tooling//:log",
        "@crates_score_tooling//:serde",
        "@crates_score_tooling//:serde_json",
    ],
)

rust_test(
    name = "puml_cli_test",
    srcs = _SRCS,
    crate_root = "src/main.rs",
    deps = [
        "//plantuml/parser/puml_api",
        "//plantuml/parser/puml_fta",
        "//plantuml/parser/puml_idmap",
        "//plantuml/parser/puml_parser",
        "//plantuml/parser/puml_resolver",
        "//plantuml/parser/puml_utils",
        "//tools/metamodel/activity:activity_diagram",
        "//tools/metamodel/class:class_diagram",
        "//tools/metamodel/component:component_diagram",
        "//tools/metamodel/sequence:sequence_diagram",
        "//tools/serialization/flatbuffers/activity:activity_serializer",
        "//tools/serialization/flatbuffers/class:class_serializer",
        "//tools/serialization/flatbuffers/component:component_serializer",
        "//tools/serialization/flatbuffers/sequence:sequence_serializer",
        "@crates_score_tooling//:clap",
        "@crates_score_tooling//:env_logger",
        "@crates_score_tooling//:log",
        "@crates_score_tooling//:serde",
        "@crates_score_tooling//:serde_json",
    ],
)
