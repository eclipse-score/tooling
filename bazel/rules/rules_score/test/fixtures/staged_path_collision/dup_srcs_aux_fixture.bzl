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
"""Minimal test-only rule that lists the exact same file in both
`SphinxSourcesInfo.deps` and `SphinxSourcesInfo.aux_srcs`, so
dependable_element's `_process_artifact_files` stages it twice for a single
label -- once while iterating doc files, once while iterating aux files --
both times from the same source label. Used by
:staged_path_same_label_collision_repro in test/BUILD to exercise
`_check_staged_path`'s same-label collision message
(dependable_element_staged_path_collision_test.bzl).
"""

load("@score_tooling//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo")

def _dup_srcs_aux_fixture_impl(ctx):
    files = depset([ctx.file.src])
    return [
        DefaultInfo(files = files),
        SphinxSourcesInfo(srcs = files, deps = files, aux_srcs = files),
    ]

dup_srcs_aux_fixture = rule(
    implementation = _dup_srcs_aux_fixture_impl,
    attrs = {
        "src": attr.label(allow_single_file = [".rst"], mandatory = True),
    },
)
