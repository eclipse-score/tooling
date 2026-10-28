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

load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "sphinx_module")

package(default_visibility = ["//:__pkg__"])

# Deliberately named the same as the module in //fixtures/same_name_modules/pkg_b --
# see :same_name_modules_lib in test/BUILD.
sphinx_module(
    name = "dup_lib",
    testonly = True,
    srcs = ["index.rst"],
    index = "index.rst",
)
