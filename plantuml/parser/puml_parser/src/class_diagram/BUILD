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
    name = "puml_parser_class_files",
    srcs = [
        "src/class_ast.rs",
        "src/class_parser.rs",
        "src/class_traits.rs",
        "src/lib.rs",
        "src/source_map.rs",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
)

rust_library(
    name = "puml_parser_class",
    srcs = [":puml_parser_class_files"],
    compile_data = [
        "//plantuml/parser/puml_parser/src/grammar:class_grammar",
    ],
    crate_name = "class_parser",
    crate_root = "src/lib.rs",
    proc_macro_deps = [
        "@crates//:pest_derive",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_parser:parser_core",
        "//plantuml/parser/puml_utils",
        "@crates//:pest",
        "@crates//:serde",
        "@crates//:thiserror",
    ],
)

rust_test(
    name = "puml_parser_class_unit_test",
    crate = ":puml_parser_class",
)

rust_test(
    name = "puml_parser_class_integration_test",
    srcs = ["test/integration_test.rs"],
    args = [
        "--nocapture",
    ],
    data = ["//plantuml/parser/puml_parser/tests/class_diagram:class_diagram_files"],
    deps = [
        ":puml_parser_class",
        "//plantuml/parser/integration_test:test_framework",
        "//plantuml/parser/puml_parser:parser_core",
        "//plantuml/parser/puml_utils",
    ],
)
