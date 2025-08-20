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
load("@aspect_rules_py//py:defs.bzl", "py_binary")

def cli_helper(name, visibility):
    py_binary(
        name = name,
        srcs = ["@score_cli_helper//tool:cli_help_lib"],
        visibility = visibility,
    )

    native.alias(
        name = "help",
        actual = ":" + name,
        visibility = visibility,
        tags = [
            "cli_help=Output all bazel targets with cli_help tag:\n" + \
            "bazel run //:help"
        ],
    )
