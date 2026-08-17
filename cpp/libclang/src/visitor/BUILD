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
        "src/clang_adapter/mod.rs",
        "src/clang_adapter/scope.rs",
        "src/clang_adapter/source_filter.rs",
        "src/clang_adapter/source_location.rs",
        "src/class_relationship_resolver.rs",
        "src/class_visitor.rs",
        "src/context.rs",
        "src/enum_visitor.rs",
        "src/function_visitor.rs",
        "src/lib.rs",
        "src/types/mod.rs",
        "src/types/renderer.rs",
        "src/types/resolver.rs",
        "src/visitor.rs",
    ],
    visibility = ["//cpp/libclang:__subpackages__"],
    deps = [
        "//cpp/libclang/src/semantics:cpp_semantics",
        "//tools/metamodel/class:class_diagram",
        "//tools/metamodel/sequence:sequence_diagram",
        "@crates//:clang",
        "@crates//:log",
        "@crates//:serde",
    ],
)

rust_test(
    name = "visitor_unit_tests",
    srcs = glob(["src/**/*.rs"]),
    crate_root = "src/lib.rs",
    deps = [
        "//cpp/libclang/src/semantics:cpp_semantics",
        "//tools/metamodel/class:class_diagram",
        "//tools/metamodel/sequence:sequence_diagram",
        "@crates//:clang",
        "@crates//:log",
        "@crates//:serde",
    ],
)
