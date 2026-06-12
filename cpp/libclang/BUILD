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
load("@bazel_skylib//rules:common_settings.bzl", "string_flag")
load("@rules_rust//rust:defs.bzl", "rust_binary")

string_flag(
    name = "log_level",
    build_setting_default = "error",
    values = [
        "error",
        "warn",
        "info",
        "debug",
        "trace",
    ],
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "clang_rs_parser",
    srcs = ["src/main.rs"],
    data = [
        "@llvm_toolchain_llvm//:libclang",
    ],
    env = {
        "LIBCLANG_PATH": "$(rootpath @llvm_toolchain_llvm//:libclang)",
    },
    visibility = ["//visibility:public"],
    deps = [
        "//cpp/libclang/src/utils",
        "//cpp/libclang/src/visitor:visit_tu",
        "@crates//:clang",
        "@crates//:clap",
        "@crates//:env_logger",
        "@crates//:log",
        "@crates//:serde_json",
    ],
)
