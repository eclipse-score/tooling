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
load("@rules_rust//rust:defs.bzl", "rust_test")
load("//cpp/libclang:cpp_parser.bzl", "CppParserInfo", "cpp_parser")

def _cpp_parser_debug_json_impl(ctx):
    debug_json = ctx.attr.parser[CppParserInfo].debug_json
    if not debug_json:
        fail("cpp_parser_debug_json requires a cpp_parser target with emit_debug_json = True")

    return [
        DefaultInfo(
            files = depset([debug_json]),
            runfiles = ctx.runfiles(files = [debug_json]),
        ),
    ]

cpp_parser_debug_json = rule(
    implementation = _cpp_parser_debug_json_impl,
    attrs = {
        "parser": attr.label(
            mandatory = True,
            providers = [CppParserInfo],
        ),
    },
)

def cpp_parser_integration_test(
        name,
        target,
        expected_output,
        srcs = None,
        extra_args = None,
        visibility = None):
    if srcs == None:
        srcs = ["run_test.rs"]
    if extra_args == None:
        extra_args = []
    if visibility == None:
        visibility = ["//cpp/libclang/integration_test:__pkg__"]

    native.filegroup(
        name = "expected_output",
        srcs = expected_output,
        visibility = visibility,
    )

    cpp_parser(
        name = "parser",
        emit_debug_json = True,
        extra_args = extra_args,
        target = target,
        visibility = visibility,
    )

    cpp_parser_debug_json(
        name = "debug_json",
        parser = ":parser",
        visibility = visibility,
    )

    rust_test(
        name = name,
        srcs = srcs,
        data = [
            ":expected_output",
            ":debug_json",
        ],
        env = {
            "DEBUG_JSON_OUTPUT_PATH": "$(rootpath :debug_json)",
            "EXPECTED_OUTPUT_PATH": "$(rootpath :expected_output)",
        },
        visibility = visibility,
        deps = [
            "//cpp/libclang/integration_test:libclang_test_framework",
        ],
    )
