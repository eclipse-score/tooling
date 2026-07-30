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

# Only directories following the "score-*" naming convention are considered
# skills that are shared with downstream repositories via //:sync_skills.
# Skills that do not follow this convention (e.g. "rules-score") are kept
# local to this repository and are not distributed.
filegroup(
    name = "skills",
    srcs = glob(
        ["score-*/**"],
        allow_empty = True,
    ),
)
