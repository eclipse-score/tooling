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
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library")

rust_library(
    name = "puml_parser_sequence",
    srcs = glob(
        ["src/*.rs"],
        exclude = ["src/*_main.rs"],
    ),
    compile_data = [
        "//plantuml/parser/puml_parser/src/grammar:sequence_grammar",
    ],
    crate_name = "sequence_parser",
    proc_macro_deps = [
        "@crates//:pest_derive",
    ],
    visibility = ["//visibility:public"],
    deps = [
        "//plantuml/parser/puml_parser:parser_core",
        "//plantuml/parser/puml_utils",
        "@crates//:pest",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

# Job 1: Parse PUML file with syntax parser, output JSON
rust_binary(
    name = "syntax_parse",
    srcs = ["src/syntax_parse_main.rs"],
    visibility = ["//visibility:public"],
    deps = [
        ":puml_parser_sequence",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
