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

load(":manual_analysis.bzl", "ManualAnalysisContextInfo")

def _manual_analysis_context_from_filegroup_impl(ctx):
    files = ctx.attr.filegroup[DefaultInfo].files
    return [
        DefaultInfo(files = files),
        ManualAnalysisContextInfo(
            files = files,
            rules = depset(),
        ),
    ]

manual_analysis_context_from_filegroup = rule(
    doc = "Creates ManualAnalysisContextInfo from a filegroup-like target.",
    implementation = _manual_analysis_context_from_filegroup_impl,
    attrs = {
        "filegroup": attr.label(
            doc = "Target that exposes files via DefaultInfo.",
            mandatory = True,
            allow_files = True,
        ),
    },
)
