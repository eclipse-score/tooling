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

# A fragment is checked against the package of the target, so the fixture
# needs a package other than the workspace root, where every file is inside.
load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "architectural_design")

package(default_visibility = ["//:__pkg__"])

# Regression fixture: a fragment of another package must fail analysis;
# see :includes_foreign_package_fails_test in test/BUILD.
architectural_design(
    name = "arch_design_includes_foreign_package_repro",
    dynamic = [
        "//:fixtures/includes/dynamic/main_seq.puml",
        "//fixtures/includes/other_pkg:frag.iuml",
    ],
    tags = ["manual"],
)

# Regression fixture: with a fragment in the target, a diagram of another
# package must fail analysis; see :includes_foreign_diagram_fails_test.
architectural_design(
    name = "arch_design_includes_foreign_diagram_repro",
    dynamic = [
        "//:fixtures/includes/dynamic/main_seq.puml",
        "local.iuml",
    ],
    tags = ["manual"],
)
