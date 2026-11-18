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

load("@rules_python//python:defs.bzl", "py_test")
load("@trlc//:trlc.bzl", "trlc_requirements", "trlc_requirements_test")

# ASIL non-degradation checks of FeatReq and CompReq: a requirement must not
# have a lower ASIL than any of its upstream requirements.

_SPEC = ["//bazel/rules/rules_score/trlc/config:score_requirements_model"]

# Single-quoted: Bazel shell-tokenizes args.
_ASIL_ERROR = "'ASIL of a derived requirement must not be lower than that of its upstream requirement'"

trlc_requirements(
    name = "upstream",
    srcs = ["upstream.trlc"],
    spec = _SPEC,
)

trlc_requirements(
    name = "valid",
    srcs = ["valid.trlc"],
    spec = _SPEC,
    deps = [":upstream"],
)

trlc_requirements(
    name = "invalid_feat",
    srcs = ["invalid_feat.trlc"],
    spec = _SPEC,
    deps = [":upstream"],
)

trlc_requirements(
    name = "invalid_comp",
    srcs = ["invalid_comp.trlc"],
    spec = _SPEC,
    deps = [
        ":upstream",
        ":valid",
    ],
)

trlc_requirements_test(
    name = "asil_valid_test",
    reqs = [":valid"],
)

py_test(
    name = "asil_invalid_feat_test",
    srcs = ["expect_check_errors_test.py"],
    args = [
        "--message",
        _ASIL_ERROR,
        "--count",
        "4",
        "$(locations :invalid_feat)",
    ],
    data = [":invalid_feat"],
    main = "expect_check_errors_test.py",
    deps = ["@trlc//trlc"],
)

py_test(
    name = "asil_invalid_comp_test",
    srcs = ["expect_check_errors_test.py"],
    args = [
        "--message",
        _ASIL_ERROR,
        "--count",
        "6",
        "$(locations :invalid_comp)",
    ],
    data = [":invalid_comp"],
    main = "expect_check_errors_test.py",
    deps = ["@trlc//trlc"],
)
