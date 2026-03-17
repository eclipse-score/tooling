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
load("@rules_rust//rust:defs.bzl", "rust_library")

filegroup(
    name = "puml_parser_component_files",
    srcs = [
        "src/component_ast.rs",
        "src/component_parser.rs",
        "src/lib.rs",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
)

rust_library(
    name = "puml_parser_component",
    srcs = [":puml_parser_component_files"],
    compile_data = [
        "//plantuml/parser/puml_parser/src/grammar:component_grammar",
    ],
    crate_name = "component_parser",
    crate_root = "src/lib.rs",
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
    ],
)
