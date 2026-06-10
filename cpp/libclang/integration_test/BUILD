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
    name = "libclang_test_framework",
    srcs = ["test_framework.rs"],
    crate_name = "test_framework",
    data = [
        "//cpp/libclang:clang_rs_parser",
        "@llvm_toolchain_llvm//:lib/libclang.so",
    ],
    visibility = ["//cpp/libclang:__subpackages__"],
    deps = [
        "@crates//:assert-json-diff",
        "@crates//:serde_json",
    ],
)

cc_binary(
    name = "simple_sequence",
    srcs = ["simple_sequence.cpp"],
)
