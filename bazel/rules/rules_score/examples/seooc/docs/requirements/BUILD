# *******************************************************************************
# Copyright (c) 2024 Contributors to the Eclipse Foundation
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

load("@trlc//:trlc.bzl", "trlc_requirements", "trlc_requirements_test")
load("//bazel/rules/rules_score:rules_score.bzl", "component_requirements", "feature_requirements")
load("//validation/ai_checker:ai_checker.bzl", "trlc_requirements_ai_test")

trlc_requirements(
    name = "assumed_system_requirements",
    srcs = [
        "assumed_system_requirements.trlc",
    ],
    spec = [
        "//bazel/rules/rules_score/trlc/config:score_requirements_model",
    ],
    visibility = ["//visibility:public"],
)

trlc_requirements_test(
    name = "assumed_system_requirements_test",
    reqs = [
        ":assumed_system_requirements",
    ],
    visibility = ["//visibility:public"],
)

trlc_requirements(
    name = "feature_requirements_trlc",
    srcs = [
        "feature_requirements.trlc",
    ],
    spec = [
        "//bazel/rules/rules_score/trlc/config:score_requirements_model",
    ],
    visibility = ["//visibility:public"],
    deps = [
        ":assumed_system_requirements",
    ],
)

feature_requirements(
    name = "feature_requirements",
    srcs = [":feature_requirements_trlc"],
    visibility = ["//visibility:public"],
)

trlc_requirements_ai_test(
    name = "feature_requirements_ai_test",
    reqs = [":feature_requirements_trlc"],
    tags = ["manual"],
)

trlc_requirements(
    name = "component_requirements_trlc",
    srcs = [
        "component_requirements.trlc",
    ],
    spec = [
        "//bazel/rules/rules_score/trlc/config:score_requirements_model",
    ],
    visibility = ["//visibility:public"],
    deps = [
        ":assumed_system_requirements",
        ":feature_requirements_trlc",
    ],
)

component_requirements(
    name = "component_requirements",
    srcs = [":component_requirements_trlc"],
    visibility = ["//visibility:public"],
)

trlc_requirements(
    name = "component_requirements_extra_trlc",
    srcs = [
        "component_requirements_extra.trlc",
    ],
    spec = [
        "//bazel/rules/rules_score/trlc/config:score_requirements_model",
    ],
    visibility = ["//visibility:public"],
    deps = [
        ":assumed_system_requirements",
        ":feature_requirements_trlc",
    ],
)

component_requirements(
    name = "component_requirements_extra",
    srcs = [":component_requirements_extra_trlc"],
    visibility = ["//visibility:public"],
)

trlc_requirements_ai_test(
    name = "component_requirements_ai_test",
    reqs = [":component_requirements_trlc"],
    tags = ["manual"],
)
