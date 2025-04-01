# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
load("@rules_shell//shell:sh_binary.bzl", "sh_binary")

def setup_starpls():
    """Sets up starpls binary for use in the current repository."""
    # We need this genrule to download the binary
    native.genrule(
        name = "download_starpls",
        outs = ["starpls_downloaded"],
        cmd = """
            curl -L "https://github.com/withered-magic/starpls/releases/download/0.1.21/starpls-linux-amd64" -o "$@" && \
            chmod +x "$@"
        """,
        visibility = ["//visibility:public"],
    )
    
    # Create the binary target that can be referenced
    sh_binary(
        name = "starpls",
        srcs = [":starpls_downloaded"],
        visibility = ["//visibility:public"],
    )



# load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")
#
# def _starpls_binary_impl(ctx):
#     """Implementation of the starpls_binary rule."""
#     # Download the binary
#     ctx.download(
#         url = "https://github.com/withered-magic/starpls/releases/download/0.1.21/starpls-linux-amd64",
#         output = "starpls",
#         executable = True,
#         sha256 = "45692ecb9d94a19a15b1e7b240acdff5702f78cd22188dac41e1879cb8bdcdcf",
#     )
#
#     # Create a BUILD file that exposes the binary
#     ctx.file("BUILD.bazel", """
# load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
#
# sh_binary(
#     name = "starpls",
#     srcs = ["starpls"],
#     visibility = ["//visibility:public"],
# )
# """)
#
# starpls_binary = rule(
#     implementation = _starpls_binary_impl,
# )
