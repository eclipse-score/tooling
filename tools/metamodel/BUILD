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

package(default_visibility = ["//visibility:public"])

# Class Diagram Metamodel Library (shared types for use by other tools)
rust_library(
    name = "class_diagram",
    srcs = ["src/class_logic.rs"],
    crate_name = "class_diagram",
    crate_root = "src/class_logic.rs",
    edition = "2021",
    deps = [
        "@crates//:serde",
        "@crates//:thiserror",
    ],
)

# Metamodel tests
rust_test(
    name = "class_diagram_test",
    crate = ":class_diagram",
    deps = [
        "@crates//:serde_json",
    ],
)

# Raw FlatBuffer schema files (for tools that invoke flatc directly)
filegroup(
    name = "schemas",
    srcs = ["schema/class_diagram.fbs"],
)
