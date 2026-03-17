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
    name = "puml_parser_core",
    srcs = [
        "src/common_ast.rs",
        "src/common_parser.rs",
        "src/error.rs",
        "src/lib.rs",
    ],
    compile_data = [
        "//plantuml/parser/puml_parser/src/grammar:grammar_files",
    ],
    crate_name = "parser_core",
    crate_root = "src/lib.rs",
    proc_macro_deps = [
        "@crates//:pest_derive",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_utils",
        "@crates//:pest",
        "@crates//:serde",
        "@crates//:thiserror",
    ],
)

rust_test(
    name = "common_grammar_test",
    crate = ":puml_parser_core",
)
