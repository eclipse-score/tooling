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

package(default_visibility = ["//visibility:public"])

# Sequence Diagram Metamodel Library (shared types for PlantUML sequence logic)
rust_library(
    name = "sequence_diagram",
    srcs = ["sequence_logic.rs"],
    crate_name = "sequence_logic",
    crate_root = "sequence_logic.rs",
    edition = "2021",
    deps = [
        "//tools/metamodel/common:source_location",
        "@crates//:serde",
    ],
)
