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

load(
    "@score_tooling//bazel/rules/rules_score:sphinx_toolchain.bzl",
    "sphinx_toolchain",
)
load(
    "@score_tooling//cpp/libclang:libclang_toolchain.bzl",
    "libclang_toolchain",
)

# ============================================================================
# libclang toolchain for the C++ parser used by score rules
# ============================================================================
libclang_toolchain(
    name = "some_other_library_libclang",
    cxx_builtin_include = "@llvm_toolchain_llvm//:cxx_builtin_include",
    extra_config_site = "@llvm_toolchain_llvm//:extra_config_site",
    libclang = "@llvm_toolchain_llvm//:lib/libclang.so",
)

toolchain(
    name = "some_other_library_libclang_toolchain",
    toolchain = ":some_other_library_libclang",
    toolchain_type = "@score_tooling//cpp/libclang:libclang_toolchain_type",
    visibility = ["//:__subpackages__"],
)
