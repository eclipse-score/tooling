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

rust_library(
    name = "test_framework",
    srcs = [
        "src/lib.rs",
        "src/test_error_view.rs",
        "src/test_framework.rs",
    ],
    crate_root = "src/lib.rs",
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_parser",
        "//plantuml/parser/puml_resolver",
        "//plantuml/parser/puml_utils",
        "//tools/metamodel:class_diagram",
        "@crates//:serde",
        "@crates//:serde_json",
        "@crates//:serde_yaml",
    ],
)
