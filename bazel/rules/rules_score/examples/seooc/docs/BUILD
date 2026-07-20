load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "assumptions_of_use",
    "glossary",
)

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
load("@trlc//:trlc.bzl", "trlc_requirements_test")

assumptions_of_use(
    name = "sample_aous",
    srcs = [
        "aous.trlc",
    ],
    visibility = ["//visibility:public"],
)

trlc_requirements_test(
    name = "aous_test",
    reqs = [
        ":sample_aous",
    ],
    visibility = ["//visibility:public"],
)

glossary(
    name = "glossary",
    srcs = ["glossary.rst"],
    visibility = ["//visibility:public"],
)
