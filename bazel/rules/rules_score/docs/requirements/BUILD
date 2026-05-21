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

load("@lobster//:lobster.bzl", "lobster_test", "lobster_trlc")
load("@trlc//:trlc.bzl", "trlc_requirements", "trlc_requirements_test", "trlc_rst")
load("//tools/lobster_rst_report:lobster_rst_report.bzl", "lobster_report_rst")

# ---------------------------------------------------------------------------
# TRLC Requirements
# ---------------------------------------------------------------------------

trlc_requirements(
    name = "use_cases",
    srcs = ["use_cases.trlc"],
    spec = ["//bazel/rules/rules_score/trlc/config:qualification_model"],
    deps = ["//bazel/rules/rules_score/trlc/config:tools"],
)

trlc_requirements(
    name = "potential_errors",
    srcs = ["potential_errors.trlc"],
    spec = ["//bazel/rules/rules_score/trlc/config:qualification_model"],
    deps = [":use_cases"],
)

trlc_requirements(
    name = "tool_requirements",
    srcs = ["tool_requirements.trlc"],
    spec = ["//bazel/rules/rules_score/trlc/config:qualification_model"],
    deps = [
        ":potential_errors",
        ":use_cases",
        "//bazel/rules/rules_score/trlc/config:tools",
    ],
)

trlc_requirements_test(
    name = "requirements_test",
    reqs = [
        ":tool_requirements",
    ],
)

# ---------------------------------------------------------------------------
# TRLC → RST rendering (for inclusion in Sphinx documentation)
# ---------------------------------------------------------------------------

trlc_rst(
    name = "requirements_rst",
    reqs = [
        ":use_cases",
        ":potential_errors",
        ":tool_requirements",
    ],
    visibility = ["//bazel/rules/rules_score:__pkg__"],
)

# ---------------------------------------------------------------------------
# Lobster traceability extraction
# ---------------------------------------------------------------------------

lobster_trlc(
    name = "use_cases_lobster",
    config = "//bazel/rules/rules_score/lobster/config/qualification:lobster_use_cases",
    requirements = [":use_cases"],
)

lobster_trlc(
    name = "potential_errors_lobster",
    config = "//bazel/rules/rules_score/lobster/config/qualification:lobster_potential_errors",
    requirements = [
        ":potential_errors",
        ":use_cases",
    ],
)

lobster_trlc(
    name = "tool_requirements_lobster",
    config = "//bazel/rules/rules_score/lobster/config/qualification:lobster_tool_requirements",
    requirements = [
        ":tool_requirements",
        ":potential_errors",
        ":use_cases",
    ],
)

# ---------------------------------------------------------------------------
# Lobster traceability report
# ---------------------------------------------------------------------------

lobster_test(
    name = "traceability_test",
    config = "//bazel/rules/rules_score/lobster/config/qualification:lobster_conf",
    inputs = [
        ":use_cases_lobster",
        ":potential_errors_lobster",
        ":tool_requirements_lobster",
        "//bazel/rules/rules_score:rules_score_impl",
    ],
    tags = ["manual"],
)

# ---------------------------------------------------------------------------
# Lobster RST report (for inclusion in Sphinx documentation)
# ---------------------------------------------------------------------------

lobster_report_rst(
    name = "traceability_rst",
    config = "//bazel/rules/rules_score/lobster/config/qualification:lobster_conf",
    inputs = [
        ":use_cases_lobster",
        ":potential_errors_lobster",
        ":tool_requirements_lobster",
        "//bazel/rules/rules_score:rules_score_impl",
    ],
    visibility = ["//bazel/rules/rules_score:__pkg__"],
)
