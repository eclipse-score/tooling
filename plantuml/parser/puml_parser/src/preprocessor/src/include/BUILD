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

filegroup(
    name = "include_preprocessor_files",
    srcs = [
        "include_ast.rs",
        "include_expander.rs",
        "include_parser.rs",
        "lib.rs",
        "utils.rs",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
)

rust_library(
    name = "include_preprocessor",
    srcs = [
        ":include_preprocessor_files",
    ],
    compile_data = [
        "//plantuml/parser/puml_parser/src/grammar:include_grammar",
    ],
    crate_name = "include_preprocessor",
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

rust_test(
    name = "include_preprocessor_unit_test",
    crate = ":include_preprocessor",
    deps = [
        "@crates//:once_cell",
    ],
)
