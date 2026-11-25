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

load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "assumed_system_requirements",
    "component_requirements",
    "feature_requirements",
)

package(default_visibility = ["//visibility:public"])

assumed_system_requirements(
    name = "assumed_system_requirements",
    srcs = ["asr.trlc"],
)

feature_requirements(
    name = "feature_requirements",
    srcs = ["feature_requirements.trlc"],
    deps = [":assumed_system_requirements"],
)

component_requirements(
    name = "component_requirements",
    srcs = ["component_requirements.trlc"],
    deps = [
        ":assumed_system_requirements",
        ":feature_requirements",
    ],
)

exports_files([
    "class_design.puml",
    "my_unit.cpp",
    "my_unit.h",
    "my_unit_test.cpp",
    "static_design.puml",
    "untraced_test.cpp",
])
