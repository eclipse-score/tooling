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

rust_library(
    name = "puml_fta",
    srcs = ["src/lib.rs"],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_parser:parser_core",
        "//plantuml/parser/puml_parser/src/preprocessor/src/procedure:procedure_preprocessor",
        "//plantuml/parser/puml_utils",
        "@crates//:log",
        "@crates//:serde",
        "@crates//:serde_json",
        "@crates//:thiserror",
    ],
)

rust_test(
    name = "puml_fta_test",
    crate = ":puml_fta",
    deps = [
        "//plantuml/parser/puml_parser:parser_core",
        "//plantuml/parser/puml_parser/src/preprocessor/src/procedure:procedure_preprocessor",
        "//plantuml/parser/puml_utils",
        "@crates//:log",
        "@crates//:serde",
        "@crates//:serde_json",
        "@crates//:thiserror",
    ],
)
