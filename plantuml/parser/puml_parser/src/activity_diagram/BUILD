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
    name = "puml_parser_activity_files",
    srcs = [
        "src/activity_ast.rs",
        "src/creole.rs",
        "src/activity_parser.rs",
        "src/lib.rs",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
)

rust_library(
    name = "puml_parser_activity",
    srcs = [":puml_parser_activity_files"],
    compile_data = [
        "//plantuml/parser/puml_parser/src/grammar:activity_grammar",
    ],
    crate_name = "activity_parser",
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
        "@crates//:thiserror",
    ],
)

rust_test(
    name = "activity_unit_test",
    crate = ":puml_parser_activity",
    proc_macro_deps = [
        "@crates//:pest_derive",
    ],
)

rust_test(
    name = "activity_integration_test",
    srcs = ["test/integration_test.rs"],
    args = [
        "--nocapture",
    ],
    data = ["//plantuml/parser/integration_test/activity_diagram:activity_diagram_files"],
    deps = [
        ":puml_parser_activity",
        "//plantuml/parser/integration_test:test_framework",
        "//plantuml/parser/puml_parser:parser_core",
        "//plantuml/parser/puml_utils",
    ],
)
