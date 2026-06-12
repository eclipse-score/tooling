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
    name = "sequence_fbs_codegen",
    srcs = ["sequence_diagram.fbs"],
    outs = ["sequence_diagram_generated.rs"],
    flatc_args = [],
    language_flag = "--rust",
)

rust_library(
    name = "sequence_fbs",
    srcs = [":sequence_fbs_codegen"],
    # Generated code from flatc - suppress all lints we can't fix
    rustc_flags = ["--cap-lints=allow"],
    deps = [
        "@crates//:flatbuffers",
    ],
)

rust_library(
    name = "sequence_serializer",
    srcs = ["sequence_serializer.rs"],
    crate_name = "sequence_serializer",
    crate_root = "sequence_serializer.rs",
    edition = "2021",
    deps = [
        ":sequence_fbs",
        "//tools/metamodel/sequence:sequence_diagram",
        "@crates//:flatbuffers",
    ],
)
