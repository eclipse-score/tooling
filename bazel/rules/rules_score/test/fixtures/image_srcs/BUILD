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

# This sub-package exists so that ctx.label.package == "fixtures/image_srcs"
# when feature_requirements stages image files.  subrule_trlc_image_stage
# strips package_prefix from short_path to produce package-relative paths
# (e.g. "diagrams/arch.png"); this only works when the package is not the
# workspace root ("").

load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "feature_requirements",
)
load(
    "//:requirements_image_test.bzl",
    "image_srcs_sphinx_sources_test",
    "requirements_image_test_suite",
)

package(default_visibility = ["//visibility:public"])

feature_requirements(
    name = "feat_req_with_image",
    srcs = ["//:fixtures/seooc_test/feature_requirements.trlc"],
    image_srcs = [
        "diagrams/arch.svg",
        "diagrams/arch.png",
    ],
    deps = ["//:asr_trlc"],
)

image_srcs_sphinx_sources_test(
    name = "image_srcs_sphinx_sources_test",
    target_under_test = ":feat_req_with_image",
)

requirements_image_test_suite(name = "requirements_image_tests")
