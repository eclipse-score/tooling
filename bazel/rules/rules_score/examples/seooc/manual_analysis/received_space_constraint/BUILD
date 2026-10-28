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

load("@score_tooling//manual_analysis:manual_analysis.bzl", "manual_analysis")

manual_analysis(
    name = "review",
    analysis = "analysis.yaml",
    contexts = [
        "//unit_1:manual_analysis_context",
        "//manual_analysis:received_aou_requirements_context",
        "//manual_analysis:dependency_contract_context",
    ],
    lock_file = "manual.lock",
    results_file = "results.json",
    tags = ["manual"],
    visibility = ["//:__pkg__"],
)
