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
"""Analysis tests for dependable_element's `_check_staged_path()` residual
`fail()` paths (bazel/rules/rules_score/private/dependable_element.bzl).

Both scenarios attach labels under the same artifact-type attribute
(`checklists`) so their outputs are staged into the same "checklists/"
directory, and both are only reachable because every label gets its own
"<target_name>/" subdirectory prefix (see _process_artifact_type):

- same-label: :staged_path_same_label_collision_repro (test/BUILD) attaches
  a single label whose SphinxSourcesInfo lists the exact same file in both
  `deps` and `aux_srcs` (see
  fixtures/staged_path_collision/dup_srcs_aux_fixture.bzl), so
  _process_artifact_files stages it twice -- once as a doc file, once as an
  aux file -- both times from the same source label.
- cross-label: :staged_path_cross_label_collision_repro (test/BUILD) attaches
  two distinct labels that happen to share a target *name* ("dup") from two
  different packages, so both stage to the same relative path from two
  different source labels.
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _same_label_collision_fails_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(
        env,
        "'checklists/dup_srcs_aux/content.rst' would be staged twice from",
    )
    return analysistest.end(env)

same_label_collision_fails_test = analysistest.make(
    _same_label_collision_fails_test_impl,
    expect_failure = True,
)

def _cross_label_collision_fails_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(
        env,
        "'checklists/dup/content.rst' would be staged by both",
    )
    return analysistest.end(env)

cross_label_collision_fails_test = analysistest.make(
    _cross_label_collision_fails_test_impl,
    expect_failure = True,
)
