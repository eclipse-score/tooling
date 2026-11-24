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
    "component",
    "component_requirements",
)
load("//lib:artifacts.bzl", "component_architecture_lobster", "golden_test")

# L3: architecture lobster of a component with two component requirements.
assumed_system_requirements(
    name = "asr",
    srcs = ["//lib:fixtures/asr.trlc"],
)

component_requirements(
    name = "comp_req",
    srcs = ["comp_req.trlc"],
    deps = [":asr"],
)

component(
    name = "comp",
    components = [],
    requirements = [":comp_req"],
    tags = ["manual"],
    tests = [],
)

component_architecture_lobster(
    name = "comp_architecture_lobster",
    src = ":comp",
)

# Refresh with: bazel run //artifacts/component:architecture_lobster_golden
golden_test(
    name = "architecture_lobster_golden",
    src = ":comp_architecture_lobster",
    golden = "comp_architecture.lobster.golden",
)
