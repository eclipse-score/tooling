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

load("//bazel/rules/rules_score:rules_score.bzl", "architectural_design")
load("//validation/core/integration_test:puml_fixture.bzl", "provider_fbs_fixture_bundle")

architectural_design(
    name = "design",
    internal_api = ["internal_api_diagram.puml"],
    static = ["component_diagram.puml"],
    visibility = ["//visibility:private"],
)

provider_fbs_fixture_bundle(
    name = "fbs",
    visibility = ["//visibility:public"],
    deps = [":design"],
)

filegroup(
    name = "case_data",
    srcs = [
        "expected.json",
        ":fbs",
    ],
    visibility = ["//visibility:public"],
)
