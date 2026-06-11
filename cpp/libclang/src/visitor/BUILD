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
    name = "visit_tu",
    srcs = [
        "src/class_parser_helper.rs",
        "src/class_visitor.rs",
        "src/context.rs",
        "src/enum_visitor.rs",
        "src/function_visitor.rs",
        "src/lib.rs",
        "src/visitor.rs",
    ],
    visibility = ["//cpp/libclang:__subpackages__"],
    deps = [
        "//tools/metamodel:class_diagram",
        "//tools/metamodel:sequence_diagram",
        "@crates//:clang",
        "@crates//:serde",
    ],
)

rust_test(
    name = "class_parser_helper_test",
    srcs = [
        "src/class_parser_helper.rs",
        "src/class_parser_helper_test.rs",
    ],
    deps = [
        "@crates//:clang",
        "@crates//:serde",
    ],
)
