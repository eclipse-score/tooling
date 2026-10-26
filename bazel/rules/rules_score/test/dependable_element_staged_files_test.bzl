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
"""Analysis test: a dependable_element stages files at the expected paths."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _staged_files_test_impl(ctx):
    env = analysistest.begin(ctx)
    short_paths = [
        f.short_path
        for f in analysistest.target_under_test(env)[DefaultInfo].files.to_list()
    ]
    for suffix in ctx.attr.suffixes:
        asserts.true(
            env,
            any([p.endswith(suffix) for p in short_paths]),
            "missing staged '{}' in: {}".format(suffix, short_paths),
        )
    return analysistest.end(env)

staged_files_test = analysistest.make(
    _staged_files_test_impl,
    attrs = {"suffixes": attr.string_list(mandatory = True)},
)
