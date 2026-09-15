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
"""
Regression tests proving two previously-crashing architectural_design configs
now build successfully, per plan_view_layout()'s compose/override reconciliation
(bazel/rules/rules_score/private/puml_utils.bzl):

- An authored directory `index.rst` alongside a `.puml` diagram in the same
  view (compose case): used to collide with the generated `index.rst` via a
  raw `declare_file()` "conflicting actions" error.
- An authored same-stem `overview.rst` alongside `overview.puml` (override
  case): used to collide with the generated wrapper page the same way.

Each analysistest below simply asserts `ArchitecturalDesignInfo` is present on
the target under test; if `plan_view_layout`/`_architectural_design_impl` still
raised a raw declare_file collision (or a `plan.errors` fail()), the
`architectural_design` target's own analysis would fail and the wrapping
analysistest target would fail to build -- so a passing test here is itself
the regression proof, not just the assertion's literal content.
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@score_tooling//bazel/rules/rules_score:providers.bzl", "ArchitecturalDesignInfo")

def _puml_layout_crash_repro_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        ArchitecturalDesignInfo in target_under_test,
        "Expected architectural_design to provide ArchitecturalDesignInfo " +
        "(i.e. to have built successfully at all)",
    )

    return analysistest.end(env)

puml_layout_crash_repro_test = analysistest.make(
    impl = _puml_layout_crash_repro_test_impl,
)
