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
load("@pip_tooling//:requirements.bzl", "requirement")
load("@rules_python//python:defs.bzl", "py_test")

# L0 tool unit tests of //bazel/rules/rules_score:* (see docs/test_strategy.rst).
# Each test is run by pytest through pytest_main.py.

py_test(
    name = "safety_analysis_assembler_test",
    size = "small",
    srcs = [
        "pytest_main.py",
        "safety_analysis_assembler_test.py",
    ],
    args = ["$(location safety_analysis_assembler_test.py)"],
    main = "pytest_main.py",
    deps = [
        requirement("pytest"),
        "//bazel/rules/rules_score:safety_analysis_assembler",
    ],
)

py_test(
    name = "aou_forwarding_to_lobster_test",
    size = "small",
    srcs = [
        "aou_forwarding_to_lobster_test.py",
        "pytest_main.py",
    ],
    args = ["$(location aou_forwarding_to_lobster_test.py)"],
    main = "pytest_main.py",
    deps = [
        requirement("pytest"),
        "//bazel/rules/rules_score:aou_forwarding_to_lobster",
        "@lobster//lobster/common",
    ],
)

py_test(
    name = "asil_coverage_check_test",
    size = "small",
    srcs = [
        "asil_coverage_check_test.py",
        "pytest_main.py",
    ],
    args = ["$(location asil_coverage_check_test.py)"],
    data = ["//bazel/rules/rules_score/trlc/config:score_requirements_model"],
    env = {"SCORE_REQUIREMENTS_MODEL": "$(rlocationpaths //bazel/rules/rules_score/trlc/config:score_requirements_model)"},
    main = "pytest_main.py",
    deps = [
        requirement("pytest"),
        "//bazel/rules/rules_score:trlc_check_runner",
        "@rules_python//python/runfiles",
    ],
)

py_test(
    name = "merge_lobster_items_test",
    size = "small",
    srcs = [
        "merge_lobster_items_test.py",
        "pytest_main.py",
    ],
    args = ["$(location merge_lobster_items_test.py)"],
    main = "pytest_main.py",
    deps = [
        requirement("pytest"),
        "//bazel/rules/rules_score:merge_lobster_items",
        "@lobster//lobster/common",
    ],
)

py_test(
    name = "sphinx_html_merge_test",
    size = "small",
    srcs = [
        "pytest_main.py",
        "sphinx_html_merge_test.py",
    ],
    args = ["$(location sphinx_html_merge_test.py)"],
    main = "pytest_main.py",
    deps = [
        requirement("pytest"),
        "//bazel/rules/rules_score:sphinx_html_merge_lib",
    ],
)

py_test(
    name = "bazel_sphinx_needs_test",
    size = "small",
    srcs = [
        "bazel_sphinx_needs_test.py",
        "pytest_main.py",
    ],
    args = ["$(location bazel_sphinx_needs_test.py)"],
    main = "pytest_main.py",
    deps = [
        requirement("pytest"),
        "//bazel/rules/rules_score:bazel_sphinx_needs",
    ],
)

py_test(
    name = "sphinx_module_ext_test",
    size = "small",
    srcs = [
        "pytest_main.py",
        "sphinx_module_ext_test.py",
    ],
    args = ["$(location sphinx_module_ext_test.py)"],
    main = "pytest_main.py",
    deps = [
        requirement("pytest"),
        "//bazel/rules/rules_score:sphinx_module_ext",
    ],
)

py_test(
    name = "rst_to_trlc_test",
    size = "small",
    srcs = [
        "pytest_main.py",
        "rst_to_trlc_test.py",
    ],
    args = ["$(location rst_to_trlc_test.py)"],
    main = "pytest_main.py",
    deps = [
        requirement("pytest"),
        "//bazel/rules/rules_score:rst_to_trlc_lib",
    ],
)

# The requirement pages depend on the pinned trlc rendering Markdown images.
py_test(
    name = "trlc_rst_rendering_contract_test",
    size = "small",
    srcs = [
        "pytest_main.py",
        "trlc_rst_rendering_contract_test.py",
    ],
    args = ["$(location trlc_rst_rendering_contract_test.py)"],
    data = [
        "fixtures/image_requirements/image_requirements.trlc",
        "fixtures/image_requirements/schema.rsl",
    ],
    env = {
        "IMAGE_REQUIREMENTS_RSL": "$(rlocationpath fixtures/image_requirements/schema.rsl)",
        "IMAGE_REQUIREMENTS_TRLC": "$(rlocationpath fixtures/image_requirements/image_requirements.trlc)",
    },
    main = "pytest_main.py",
    deps = [
        requirement("pytest"),
        "@rules_python//python/runfiles",
        "@trlc//tools/trlc_rst:trlc_rst_lib",
    ],
)

test_suite(
    name = "tests",
    tests = [
        ":aou_forwarding_to_lobster_test",
        ":asil_coverage_check_test",
        ":bazel_sphinx_needs_test",
        ":merge_lobster_items_test",
        ":rst_to_trlc_test",
        ":safety_analysis_assembler_test",
        ":sphinx_html_merge_test",
        ":sphinx_module_ext_test",
        ":trlc_rst_rendering_contract_test",
    ],
)
