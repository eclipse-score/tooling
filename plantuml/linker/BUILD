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
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_test")

rust_binary(
    name = "linker",
    srcs = ["src/main.rs"],
    crate_root = "src/main.rs",
    visibility = ["//visibility:public"],
    deps = [
        "//plantuml/parser/puml_serializer/src/fbs:component_fbs",
        "@crates//:clap",
        "@crates//:flatbuffers",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

rust_test(
    name = "linker_test",
    crate = ":linker",
)
