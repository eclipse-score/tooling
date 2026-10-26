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

load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "unit_design")

package(default_visibility = ["//:__pkg__"])

# Sub-package: both diagrams are staged by basename next to the unit page, so
# they collide -- see :unit_design_collision_fails_test in test/BUILD.
unit_design(
    name = "design",
    static = [
        "a/class_diagram.puml",
        "b/class_diagram.puml",
    ],
    tags = ["manual"],
)
