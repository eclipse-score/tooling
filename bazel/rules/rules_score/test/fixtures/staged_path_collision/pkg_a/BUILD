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

load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "glossary")

package(default_visibility = ["//:__pkg__"])

# Deliberately named the same as //fixtures/staged_path_collision/pkg_b:dup
# -- see :staged_path_cross_label_collision_repro in test/BUILD, which attaches
# both under the same artifact-type attribute to exercise _check_staged_path's
# cross-label collision message (two different labels sharing a target name
# from different packages).
glossary(
    name = "dup",
    srcs = ["content.rst"],
)
