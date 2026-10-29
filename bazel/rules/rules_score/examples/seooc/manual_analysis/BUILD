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

load("@score_tooling//manual_analysis:context_from_filegroup.bzl", "manual_analysis_context_from_filegroup")

package(default_visibility = ["//manual_analysis:__subpackages__"])

manual_analysis_context_from_filegroup(
    name = "received_aou_requirements_context",
    filegroup = "//docs/requirements:received_aou_manual_analysis_sources",
)

manual_analysis_context_from_filegroup(
    name = "dependency_contract_context",
    filegroup = "@some_other_library//:input_pointer_review_sources",
)
