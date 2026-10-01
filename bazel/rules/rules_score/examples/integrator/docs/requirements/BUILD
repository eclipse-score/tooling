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

load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "assumed_system_requirements", "component_requirements", "feature_requirements")

assumed_system_requirements(
    name = "assumed_system_requirements",
    srcs = [
        "assumed_system_requirements.trlc",
    ],
    visibility = ["//visibility:public"],
)

feature_requirements(
    name = "feature_requirements",
    srcs = [
        "feature_requirements.trlc",
    ],
    visibility = ["//visibility:public"],
    deps = [
        ":assumed_system_requirements",
    ],
)

component_requirements(
    name = "component_requirements",
    testonly = True,
    srcs = [
        "component_requirements.trlc",
    ],
    visibility = ["//visibility:public"],
    deps = [
        ":feature_requirements",
        # A single dep on the immediate dependable_element dependency is
        # enough to resolve both AoU references below -- it aggregates its
        # own AoU (SampleType.SampleAoU) and the AoU it chain-forwards from
        # its own dependency (OtherLibrary.TimingConstraint) via
        # TrlcProviderInfo. No direct visibility to
        # @some_other_library//:other_library_aous (the AoU's ultimate
        # origin, several `deps` hops away) is needed.
        "@seooc//:safety_software_seooc_example",
    ],
)
