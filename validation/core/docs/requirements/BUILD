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

load("@trlc//:trlc.bzl", "trlc_requirements", "trlc_rst")

trlc_requirements(
    name = "tool_requirements",
    srcs = ["tool_requirements.trlc"],
    spec = ["//bazel/rules/rules_score/trlc/config:qualification_model"],
    deps = [
        "//bazel/rules/rules_score/docs/requirements:use_cases",
        "//bazel/rules/rules_score/trlc/config:tools",
    ],
)

trlc_rst(
    name = "requirements_rst",
    reqs = [":tool_requirements"],
    visibility = ["//bazel/rules/rules_score:__pkg__"],
    deps = [
        "//bazel/rules/rules_score/docs/requirements:use_cases",
        "//bazel/rules/rules_score/trlc/config:tools",
    ],
)
