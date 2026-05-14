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

load("//bazel/rules/rules_score:rules_score.bzl", "architectural_design", "unit_design")
load("//validation/core/integration_test:puml_fixture.bzl", "provider_fbs_fixture_bundle")

architectural_design(
    name = "component_design",
    static = [
        "component_diagram_part1.puml",
        "component_diagram_part2.puml",
    ],
    visibility = ["//visibility:private"],
)

unit_design(
    name = "class_design",
    static = [
        "class_diagram_part1.puml",
        "class_diagram_part2.puml",
    ],
    visibility = ["//visibility:private"],
)

provider_fbs_fixture_bundle(
    name = "fbs",
    visibility = ["//visibility:public"],
    deps = [
        ":class_design",
        ":component_design",
    ],
)

filegroup(
    name = "case_data",
    srcs = [
        "expected.json",
        ":fbs",
    ],
    visibility = ["//visibility:public"],
)
