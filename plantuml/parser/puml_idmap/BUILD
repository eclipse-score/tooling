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
    name = "puml_idmap",
    srcs = ["src/lib.rs"],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//tools/metamodel/class:class_diagram",
        "//tools/metamodel/component:component_diagram",
        "//tools/metamodel/sequence:sequence_diagram",
        "@crates//:log",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

rust_test(
    name = "puml_idmap_test",
    crate = ":puml_idmap",
    deps = [
        "//tools/metamodel/class:class_diagram",
        "//tools/metamodel/component:component_diagram",
        "//tools/metamodel/sequence:sequence_diagram",
    ],
)
