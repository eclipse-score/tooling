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
load("@flatbuffers//:build_defs.bzl", "flatbuffer_library_public")
load("@rules_rust//rust:defs.bzl", "rust_library")

flatbuffer_library_public(
    name = "component_fbs_codegen",
    srcs = [
        "component.fbs",
    ],
    outs = [
        "component_generated.rs",
    ],
    flatc_args = [],
    language_flag = "--rust",
    visibility = ["//plantuml/parser:__subpackages__"],
)

rust_library(
    name = "component_fbs",
    srcs = [
        ":component_fbs_codegen",
    ],
    # Generated code from flatc - suppress clippy warnings we can't fix
    rustc_flags = [
        "--allow=unused_imports",
        "--allow=clippy::extra-unused-lifetimes",
        "--allow=clippy::missing-safety-doc",
        "--allow=clippy::needless-lifetimes",
    ],
    visibility = [
        "//plantuml/parser:__subpackages__",
    ],
    deps = [
        "@crates//:flatbuffers",
    ],
)
