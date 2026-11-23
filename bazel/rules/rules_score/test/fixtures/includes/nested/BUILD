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

# Fragments of a package below this one; the diagram includes `frags/a.iuml`,
# which includes `b.iuml` beside it.
load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "architectural_design")

package(default_visibility = ["//:__pkg__"])

architectural_design(
    name = "arch_design_includes_nested",
    dynamic = [
        "main.puml",
        "//fixtures/includes/nested/frags:a.iuml",
        "//fixtures/includes/nested/frags:b.iuml",
    ],
    # The fixture is not checked against a component diagram.
    maturity = "development",
)
