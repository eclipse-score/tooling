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
    name = "puml_resolver_component",
    srcs = [
        "src/component_logic.rs",
        "src/component_resolver.rs",
        "src/lib.rs",
    ],
    crate_name = "component_resolver",
    crate_root = "src/lib.rs",
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_parser:component_diagram",
        "//plantuml/parser/puml_resolver:resolver_traits",
        "@crates//:log",
        "@crates//:serde",
        "@crates//:thiserror",
    ],
)

rust_test(
    name = "puml_resolver_component_it",
    srcs = [
        "tests/component_resolver_test.rs",
    ],
    crate_root = "tests/component_resolver_test.rs",
    data = [
        "//plantuml/parser/integration_test/component_diagram:component_diagram_files",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        ":puml_resolver_component",
        "//plantuml/parser/integration_test:test_framework",
        "//plantuml/parser/puml_parser:component_diagram",
        "//plantuml/parser/puml_parser:parser_core",
        "//plantuml/parser/puml_resolver:resolver_traits",
        "//plantuml/parser/puml_utils",
        "@crates//:serde",
        "@crates//:serde_json",
        "@crates//:thiserror",
    ],
)
