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
"""Tool-qualification traceability check including tool tests."""

load("@lobster//:lobster.bzl", "lobster_test")

def _tool_traceability_test_impl(name, visibility, tool_tests, tags):
    for t in tool_tests:
        if t.name != "tool_tests":
            fail("tool_tests must be named 'tool_tests', as the lobster config refers to tool_tests.lobster: " + str(t))

    # Symbolic macro: private requirement targets stay usable from other modules.
    lobster_test(
        name = name,
        config = Label("//bazel/rules/rules_score/lobster/config/qualification:lobster_conf"),
        inputs = [
            Label(":use_cases_lobster"),
            Label(":potential_errors_lobster"),
            Label(":tool_requirements_lobster"),
            Label("//bazel/rules/rules_score:rules_score_impl"),
        ] + tool_tests,
        tags = tags,
        visibility = visibility,
    )

tool_traceability_test = macro(
    implementation = _tool_traceability_test_impl,
    attrs = {
        "tags": attr.string_list(configurable = False),
        "tool_tests": attr.label_list(
            doc = "The lobster_gtest target of the traced tool tests; must be named tool_tests.",
            mandatory = True,
            configurable = False,
        ),
    },
)
