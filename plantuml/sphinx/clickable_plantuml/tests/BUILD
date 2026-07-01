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
load("@pip_rules_score//:requirements.bzl", "requirement")
load("@score_tooling//python_basics:defs.bzl", "score_py_pytest")

score_py_pytest(
    name = "unit_tests",
    srcs = ["test_clickable_plantuml.py"],
    deps = [
        "//plantuml/sphinx/clickable_plantuml",
        requirement("docutils"),
        requirement("sphinx"),
    ],
)
