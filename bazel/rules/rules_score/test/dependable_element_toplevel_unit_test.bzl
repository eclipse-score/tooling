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
"""Analysis test: units listed directly in dependable_element.components are linked in a toctree."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _toplevel_unit_linked_test_impl(ctx):
    env = analysistest.begin(ctx)

    actions = [
        a
        for a in analysistest.target_actions(env)
        if a.mnemonic == "TemplateExpand" and a.outputs.to_list()[0].basename == "components.rst"
    ]
    asserts.equals(env, 1, len(actions))
    if len(actions) == 1:
        entries = {s: v for s, v in actions[0].substitutions.items()}["{entries}"]
        asserts.true(
            env,
            "units/{}/index".format(ctx.attr.unit_name) in entries,
            "top-level unit page missing from components toctree: " + entries,
        )
    return analysistest.end(env)

toplevel_unit_linked_test = analysistest.make(
    _toplevel_unit_linked_test_impl,
    attrs = {"unit_name": attr.string(mandatory = True)},
)
