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

load("//bazel/rules/rules_score:rules_score.bzl", "assumed_system_requirements", "component_requirements", "feature_requirements")

assumed_system_requirements(
    name = "assumed_system_requirements",
    srcs = [
        "assumed_system_requirements.trlc",
    ],
    visibility = ["//visibility:public"],
)

feature_requirements(
    name = "feature_requirements",
    srcs = [
        "feature_requirements.trlc",
    ],
    visibility = ["//visibility:public"],
    deps = [
        ":assumed_system_requirements",
    ],
)

component_requirements(
    name = "component_requirements",
    srcs = [
        "component_requirements.trlc",
    ],
    visibility = ["//visibility:public"],
    deps = [
        ":feature_requirements",
    ],
)
