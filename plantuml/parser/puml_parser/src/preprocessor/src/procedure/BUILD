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
load("@rules_rust//rust:defs.bzl", "rust_library")

filegroup(
    name = "procedure_preprocessor_files",
    srcs = [
        "lib.rs",
        "procedure_ast.rs",
        "procedure_expander.rs",
        "procedure_parser.rs",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
)

rust_library(
    name = "procedure_preprocessor",
    srcs = [
        ":procedure_preprocessor_files",
    ],
    compile_data = [
        "//plantuml/parser/puml_parser/src/grammar:procedure_grammar",
    ],
    crate_name = "procedure_preprocessor",
    crate_root = "lib.rs",
    proc_macro_deps = [
        "@crates//:pest_derive",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_parser:parser_core",
        "//plantuml/parser/puml_utils",
        "@crates//:log",
        "@crates//:pest",
        "@crates//:serde",
        "@crates//:thiserror",
    ],
)
