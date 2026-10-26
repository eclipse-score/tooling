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

load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "component", "unit")

package(default_visibility = ["//:__pkg__"])

# Deliberately named the same as the targets in //fixtures/same_name_targets/pkg_b;
# only design_name tells them apart in the architecture diagram.
# See :same_name_units_lib and :same_name_components_lib in test/BUILD.
unit(
    name = "dup_unit",
    testonly = True,
    design_name = "dup_unit_a",
    maturity = "development",
    tests = [],
    unit_design = [],
    implementation = ["//:mock_lib1"],
)

component(
    name = "dup_comp",
    testonly = True,
    components = [":dup_unit"],
    design_name = "dup_comp_a",
    requirements = [
        "//:comp_req",
        "//:feat_req",
    ],
    tests = [],
)
