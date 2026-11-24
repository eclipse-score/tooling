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
load(":pytest_test.bzl", "pytest_test")

# L0 tool unit tests of //bazel/rules/rules_score:* (see docs/test_strategy.rst).

exports_files(["pytest_main.py"])

pytest_test(
    name = "safety_analysis_assembler_test",
    src = "safety_analysis_assembler_test.py",
    deps = ["//bazel/rules/rules_score:safety_analysis_assembler"],
)

pytest_test(
    name = "aou_forwarding_to_lobster_test",
    src = "aou_forwarding_to_lobster_test.py",
    deps = [
        "//bazel/rules/rules_score:aou_forwarding_to_lobster",
        "@lobster//lobster/common",
    ],
)

pytest_test(
    name = "asil_coverage_check_test",
    src = "asil_coverage_check_test.py",
    data = ["//bazel/rules/rules_score/trlc/config:score_requirements_model"],
    env = {"SCORE_REQUIREMENTS_MODEL": "$(rlocationpaths //bazel/rules/rules_score/trlc/config:score_requirements_model)"},
    deps = [
        "//bazel/rules/rules_score:trlc_check_runner",
        "@rules_python//python/runfiles",
    ],
)

pytest_test(
    name = "merge_lobster_items_test",
    src = "merge_lobster_items_test.py",
    deps = [
        "//bazel/rules/rules_score:merge_lobster_items",
        "@lobster//lobster/common",
    ],
)

pytest_test(
    name = "sphinx_html_merge_test",
    src = "sphinx_html_merge_test.py",
    deps = ["//bazel/rules/rules_score:sphinx_html_merge_lib"],
)

pytest_test(
    name = "bazel_sphinx_needs_test",
    src = "bazel_sphinx_needs_test.py",
    deps = ["//bazel/rules/rules_score:bazel_sphinx_needs"],
)

pytest_test(
    name = "sphinx_module_ext_test",
    src = "sphinx_module_ext_test.py",
    deps = ["//bazel/rules/rules_score:sphinx_module_ext"],
)

pytest_test(
    name = "rst_to_trlc_test",
    src = "rst_to_trlc_test.py",
    deps = ["//bazel/rules/rules_score:rst_to_trlc_lib"],
)

# The requirement pages depend on the pinned trlc rendering Markdown images.
pytest_test(
    name = "trlc_rst_rendering_contract_test",
    src = "trlc_rst_rendering_contract_test.py",
    data = [
        "fixtures/image_requirements/image_requirements.trlc",
        "fixtures/image_requirements/schema.rsl",
    ],
    env = {
        "IMAGE_REQUIREMENTS_RSL": "$(rlocationpath fixtures/image_requirements/schema.rsl)",
        "IMAGE_REQUIREMENTS_TRLC": "$(rlocationpath fixtures/image_requirements/image_requirements.trlc)",
    },
    deps = [
        "@rules_python//python/runfiles",
        "@trlc//tools/trlc_rst:trlc_rst_lib",
    ],
)

# Stands in for `bazel test --test_filter=test_selected`.
pytest_test(
    name = "pytest_filter_test",
    src = "fixtures/pytest_filter/filter_fixture.py",
    env = {"TESTBRIDGE_TEST_ONLY": "test_selected"},
)

test_suite(name = "tests")
