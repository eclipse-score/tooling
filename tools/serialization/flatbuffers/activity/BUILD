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

package(default_visibility = ["//visibility:public"])

flatbuffer_library_public(
    name = "activity_fbs_codegen",
    srcs = ["activity_diagram.fbs"],
    outs = ["activity_diagram_generated.rs"],
    flatc_args = [],
    language_flag = "--rust",
)

rust_library(
    name = "activity_fbs",
    srcs = [":activity_fbs_codegen"],
    # Generated code from flatc - suppress all lints we can't fix
    rustc_flags = ["--cap-lints=allow"],
    deps = [
        "@crates//:flatbuffers",
    ],
)

rust_library(
    name = "activity_serializer",
    srcs = ["activity_serializer.rs"],
    crate_name = "activity_serializer",
    crate_root = "activity_serializer.rs",
    edition = "2021",
    deps = [
        ":activity_fbs",
        "//tools/metamodel/activity:activity_diagram",
        "@crates//:flatbuffers",
    ],
)
