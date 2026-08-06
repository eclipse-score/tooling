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

load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "component_requirements")
load("@trlc//:trlc.bzl", "trlc_specification")
load(
    "//:requirements_spec_extras_test.bzl",
    "requirements_spec_extras_test_suite",
    "spec_extras_in_provider_test",
)

package(default_visibility = ["//visibility:public"])

trlc_specification(
    name = "extra_spec",
    srcs = ["extra_model.rsl"],
)

component_requirements(
    name = "spec_extras_comp_req",
    srcs = ["project_requirements.trlc"],
    spec = [":extra_spec"],
)

spec_extras_in_provider_test(
    name = "spec_extras_in_provider_test",
    target_under_test = ":spec_extras_comp_req",
)

requirements_spec_extras_test_suite(name = "requirements_spec_extras_tests")
