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
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "puml_cli",
    srcs = ["src/main.rs"],
    crate_root = "src/main.rs",
    visibility = ["//visibility:public"],
    deps = [
        "//plantuml/parser/puml_lobster",
        "//plantuml/parser/puml_parser",
        "//plantuml/parser/puml_resolver",
        "//plantuml/parser/puml_serializer",
        "//plantuml/parser/puml_utils",
        "//tools/metamodel:class_diagram",
        "@crates//:clap",
        "@crates//:env_logger",
        "@crates//:log",
        "@crates//:serde",
    ],
)
