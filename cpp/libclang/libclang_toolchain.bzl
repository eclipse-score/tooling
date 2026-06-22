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

"""Toolchain definition that supplies libclang and the C++ include paths.

score_tooling does not hard-code a specific LLVM installation for the libclang
based C++ parser. Instead, each integrating repository registers its own
`libclang_toolchain`, so it can decide which libclang/C++ standard library the
parser resolves against.
"""

LIBCLANG_TOOLCHAIN_TYPE = "//cpp/libclang:libclang_toolchain_type"

LibclangToolchainInfo = provider(
    doc = "libclang shared library and C++ include directories for the parser.",
    fields = {
        "libclang": "libclang shared library File loaded by the parser at runtime.",
        "cxx_builtin_include": "depset of files exposing the libc++ header directory " +
                               "(include/c++) and the clang resource include directory " +
                               "(lib/clang/<version>/include).",
        "extra_config_site": "depset of files exposing the arch-specific __config_site " +
                             "(include/<triple>/c++/v1/__config_site) used to locate the " +
                             "ABI include path.",
    },
)

def _libclang_toolchain_impl(ctx):
    return [
        platform_common.ToolchainInfo(
            libclang_info = LibclangToolchainInfo(
                libclang = ctx.file.libclang,
                cxx_builtin_include = depset(ctx.files.cxx_builtin_include),
                extra_config_site = depset(ctx.files.extra_config_site),
            ),
        ),
    ]

libclang_toolchain = rule(
    implementation = _libclang_toolchain_impl,
    doc = "Provides libclang and the C++ include directories required by the parser.",
    attrs = {
        "libclang": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "libclang shared library (e.g. lib/libclang.so).",
        ),
        "cxx_builtin_include": attr.label(
            allow_files = True,
            mandatory = True,
            doc = "Filegroup with the libc++ header directory (include/c++) and the " +
                  "clang resource include directory (lib/clang/<version>/include).",
        ),
        "extra_config_site": attr.label(
            allow_files = True,
            mandatory = True,
            doc = "Filegroup with the arch-specific __config_site file " +
                  "(include/<triple>/c++/v1/__config_site).",
        ),
    },
)
