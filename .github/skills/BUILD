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

package(default_visibility = ["//visibility:public"])

# Only "score-*" skills are shared with downstream repositories via
# //:sync_skills; others (e.g. "rules-score") stay local. Files are listed
# explicitly so a missing or renamed file fails the build.
filegroup(
    name = "skills",
    srcs = [
        "score-architecture/SKILL.md",
        "score-requirements/SKILL.md",
        "score-safety-analysis/SKILL.md",
        "score-testing/SKILL.md",
    ],
)
