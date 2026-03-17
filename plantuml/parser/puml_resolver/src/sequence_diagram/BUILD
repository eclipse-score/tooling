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
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")

rust_library(
    name = "puml_resolver_sequence",
    srcs = [
        "src/lib.rs",
        "src/logic_ast.rs",
        "src/logic_parser.rs",
    ],
    crate_name = "sequence_resolver",
    crate_root = "src/lib.rs",
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_parser:sequence_diagram",
        "//plantuml/parser/puml_resolver:resolver_traits",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

# Job 2: Build hierarchical tree from syntax JSON
rust_binary(
    name = "logic_parse",
    srcs = ["src/logic_parse_main.rs"],
    visibility = ["//visibility:public"],
    deps = [
        ":puml_resolver_sequence",
        "//plantuml/parser/puml_parser:sequence_diagram",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

# Test: Compare logic_parse output with expected JSON
rust_test(
    name = "logic_parse_test",
    srcs = ["test/logic_parse_test.rs"],
    data = [
        "test/logic.json",
        "test/syntax.json",
    ],
    deps = [
        ":puml_resolver_sequence",
        "//plantuml/parser/puml_parser:sequence_diagram",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
