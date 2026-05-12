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

filegroup(
    name = "puml_parser_activity_files",
    srcs = [
        "src/activity_ast.rs",
        "src/lib.rs",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
)

rust_library(
    name = "puml_parser_activity",
    srcs = [":puml_parser_activity_files"],
    crate_name = "activity_parser",
    crate_root = "src/lib.rs",
    visibility = ["//plantuml/parser:__subpackages__"],
)
