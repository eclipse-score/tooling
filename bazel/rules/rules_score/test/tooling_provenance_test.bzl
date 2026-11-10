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
"""Loading-phase tests for resolved tooling provenance."""

load("@bazel_skylib//lib:unittest.bzl", "loadingtest")
load("@score_tooling//bazel/rules/rules_score/private:tooling_provenance.bzl", "resolved_tooling_version")

def tooling_provenance_test_suite(name):
    """Check version selection independently of the consumer's identity."""
    env = loadingtest.make(name)
    loadingtest.equals(
        env,
        "resolved_tooling_version",
        "1.3.2",
        resolved_tooling_version([
            struct(name = "consumer", version = "9.0.0"),
            struct(name = "score_tooling", version = "1.3.2"),
        ]),
    )
    loadingtest.equals(
        env,
        "unversioned_tooling",
        "unavailable (unversioned module)",
        resolved_tooling_version([struct(name = "score_tooling", version = "")]),
    )
    loadingtest.equals(
        env,
        "updated_tooling_version",
        "1.4.0",
        resolved_tooling_version([struct(name = "score_tooling", version = "1.4.0")]),
    )
