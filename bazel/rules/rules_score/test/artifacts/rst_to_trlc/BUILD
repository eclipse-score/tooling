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

load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "rst_to_trlc")
load("//lib:artifacts.bzl", "golden_test")

# L3: tests of real action outputs (golden diffs, assertions on generated artifacts).
rst_to_trlc(
    name = "aou_trlc",
    srcs = ["aou.rst"],
)

# Refresh with: bazel run //artifacts/rst_to_trlc:aou_golden
golden_test(
    name = "aou_golden",
    src = ":aou_trlc",
    golden = "aou.trlc.golden",
)
