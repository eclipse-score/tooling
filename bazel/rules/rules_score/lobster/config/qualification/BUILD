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

# Lobster configuration files for tool-qualification traceability.
# lobster.conf: full chain incl. tool tests (checked from the test module).
# lobster-requirements.conf: requirements chain only (rendered in the docs).

filegroup(
    name = "lobster_conf",
    srcs = ["lobster.conf"],
    visibility = ["//bazel/rules/rules_score/docs/requirements:__pkg__"],
)

filegroup(
    name = "lobster_requirements_conf",
    srcs = ["lobster-requirements.conf"],
    visibility = ["//bazel/rules/rules_score/docs/requirements:__pkg__"],
)

filegroup(
    name = "lobster_use_cases",
    srcs = ["lobster-use-cases.yaml"],
    visibility = ["//bazel/rules/rules_score/docs/requirements:__pkg__"],
)

filegroup(
    name = "lobster_potential_errors",
    srcs = ["lobster-potential-errors.yaml"],
    visibility = ["//bazel/rules/rules_score/docs/requirements:__pkg__"],
)

filegroup(
    name = "lobster_tool_requirements",
    srcs = ["lobster-tool-requirements.yaml"],
    visibility = ["//bazel/rules/rules_score/docs/requirements:__pkg__"],
)
