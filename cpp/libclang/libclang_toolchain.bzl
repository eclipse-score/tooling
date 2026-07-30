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

"""Toolchain definition that supplies libclang and a clang cc_toolchain.

score_tooling does not hard-code a specific LLVM installation for the libclang
based C++ parser. Instead, each integrating repository registers its own
`libclang_toolchain`, so it can decide which libclang/cc_toolchain the parser
resolves against.

The parser derives its hermetic include paths, resource-dir, target triple and
standard library flags directly from the referenced `cc_toolchain` (via
cc_common), instead of hand-parsing LLVM installation file layouts.
"""

load("@rules_cc//cc/common:cc_common.bzl", "cc_common")

LIBCLANG_TOOLCHAIN_TYPE = "//cpp/libclang:libclang_toolchain_type"

LibclangToolchainInfo = provider(
    doc = "libclang shared library and clang cc_toolchain for the parser.",
    fields = {
        "libclang": "libclang shared library File loaded by the parser at runtime.",
        "cc_toolchain": "CcToolchainInfo of the clang cc_toolchain used to derive hermetic " +
                        "compiler flags (target triple, resource-dir, sysroot, stdlib, ...) " +
                        "for the parser, e.g. @llvm_toolchain//:cc-clang-x86_64-linux.",
    },
)

def _libclang_toolchain_impl(ctx):
    return [
        platform_common.ToolchainInfo(
            libclang_info = LibclangToolchainInfo(
                libclang = ctx.file.libclang,
                cc_toolchain = ctx.attr.cc_toolchain[cc_common.CcToolchainInfo],
            ),
        ),
    ]

libclang_toolchain = rule(
    implementation = _libclang_toolchain_impl,
    doc = "Provides libclang and the clang cc_toolchain required by the parser.",
    attrs = {
        "libclang": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "libclang shared library (e.g. lib/libclang.so).",
        ),
        "cc_toolchain": attr.label(
            mandatory = True,
            providers = [cc_common.CcToolchainInfo],
            cfg = "exec",
            doc = "Clang cc_toolchain used to derive hermetic compiler flags for the " +
                  "parser (e.g. @llvm_toolchain//:cc-clang-x86_64-linux). Resolved in " +
                  "the exec configuration since libclang.so is dlopen()'d in-process " +
                  "by the exec-platform clang_rs_parser binary: the derived target " +
                  "triple/sysroot/resource-dir must match the exec platform, not the " +
                  "target platform being built for. This means the parser always " +
                  "analyzes source using the exec platform's toolchain flags, even " +
                  "when cross-compiling; only target-specific -D/-I/-iquote/-isystem " +
                  "flags (CompilationFlagsInfo, derived from the parsed target's own " +
                  "CcInfo) vary per target.",
        ),
    },
)
