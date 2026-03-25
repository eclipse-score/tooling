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
load("@rules_rust//rust:defs.bzl", "rust_test")

# Test suite: Compare parsed output with expected JSON for each test pair
rust_test(
    name = "syntax_parse_test",
    srcs = ["syntax_parse_test.rs"],
    data = [
        "//plantuml/parser/integration_test/sequence_diagram:sequence_diagram_tests",
    ],
    deps = [
        "//plantuml/parser/puml_parser:parser_core",
        "//plantuml/parser/puml_parser:sequence_diagram",
        "//plantuml/parser/puml_utils",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
