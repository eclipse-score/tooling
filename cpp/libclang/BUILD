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
load(":libclang_toolchain.bzl", "libclang_toolchain")

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

# Toolchain type that supplies libclang + the C++ include paths to the parser.
# score_tooling does not register a libclang toolchain for consumers; each
# integrating repository registers its own so it controls which libclang and
# C++ standard library the parser resolves against.
toolchain_type(
    name = "libclang_toolchain_type",
    visibility = ["//visibility:public"],
)

# Default libclang toolchain used by score_tooling's own build. It wraps the
# LLVM installation that score_tooling declares as a dev dependency and is
# registered via score_tooling's .bazelrc (--extra_toolchains), so it is NOT
# inherited by repositories that depend on score_tooling.
libclang_toolchain(
    name = "score_tooling_libclang",
    cxx_builtin_include = "@llvm_toolchain_llvm//:cxx_builtin_include",
    extra_config_site = "@llvm_toolchain_llvm//:extra_config_site",
    libclang = "@llvm_toolchain_llvm//:lib/libclang.so",
    visibility = ["//visibility:public"],
)

toolchain(
    name = "score_tooling_libclang_toolchain",
    toolchain = ":score_tooling_libclang",
    toolchain_type = ":libclang_toolchain_type",
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "clang_rs_parser",
    srcs = ["src/main.rs"],
    visibility = ["//visibility:public"],
    deps = [
        "//cpp/libclang/src/utils",
        "//cpp/libclang/src/visitor:visit_tu",
        "//tools/metamodel/class:class_diagram",
        "//tools/serialization/flatbuffers/class:class_serializer",
        "@crates//:clang",
        "@crates//:clap",
        "@crates//:env_logger",
        "@crates//:log",
        "@crates//:serde_json",
    ],
)
