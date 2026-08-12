# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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

load("@trlc//:trlc.bzl", "trlc_requirements", "trlc_requirements_test", "trlc_specification")
load("//bazel/rules/rules_score/trlc/config/test:trlc_check_test.bzl", "trlc_check_test")

trlc_specification(
    name = "score_requirements_model",
    srcs = [
        "score_requirements_model.rsl",
    ],
    visibility = ["//visibility:public"],
)

# The model contains ASIL checks that crash TRLC 3.0.0's VCG (CVC5 backend)
# when --verify is used.  Use trlc_check_test (no --verify) so the model parses
# and checks evaluate at runtime without triggering the VCG crash.
trlc_check_test(
    name = "score_requirements_model_test",
    reqs = [
        ":score_requirements_model",
    ],
)

trlc_specification(
    name = "qualification_model",
    srcs = [
        "qualification_model.rsl",
    ],
    visibility = ["//visibility:public"],
)

trlc_requirements_test(
    name = "qualification_model_test",
    reqs = [
        ":qualification_model",
    ],
)

trlc_requirements(
    name = "tools",
    srcs = [
        "tools.trlc",
    ],
    spec = [":qualification_model"],
    visibility = ["//visibility:public"],
)
