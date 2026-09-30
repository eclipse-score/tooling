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

filegroup(
    name = "cross_diagram_files",
    srcs = glob([
        "**/*.json",
        "**/*.puml",
        "**/*.yaml",
    ]),
)

rust_test(
    name = "cross_diagram_test",
    srcs = ["cross_diagram_test.rs"],
    crate_root = "cross_diagram_test.rs",
    data = [
        ":cross_diagram_files",
        "//plantuml/parser/puml_cli",
    ],
    deps = [
        "//plantuml/parser/integration_test:test_framework",
        "@crates//:serde",
        "@crates//:serde_json",
        "@crates//:serde_yaml",
    ],
)
