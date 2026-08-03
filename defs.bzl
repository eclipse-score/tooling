# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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

"""Unified entrypoint for score_tooling Bazel macros & rules."""

# --- cli_helper ---
load("//cli_helper:cli_helper.bzl", _cli_helper = "cli_helper")

# --- coverage ---
load(
    "//coverage:defs.bzl",
    _score_coverage_reporter = "score_coverage_reporter",
    _score_coverage_scope = "score_coverage_scope",
)

# --- dash ---
load("//dash:dash.bzl", _dash_license_checker = "dash_license_checker")

# --- python_basics ---
load(
    "//python_basics:defs.bzl",
    _score_py_pytest = "score_py_pytest",
    _score_virtualenv = "score_virtualenv",
)

# --- starpls ---
load("//starpls:starpls.bzl", _setup_starpls = "setup_starpls")

# --- ai_checker ---
load(
    "//validation/ai_checker:ai_checker.bzl",
    _architecture_ai_test = "architecture_ai_test",
    _trlc_requirements_ai_test = "trlc_requirements_ai_test",
)

score_virtualenv = _score_virtualenv
score_py_pytest = _score_py_pytest
dash_license_checker = _dash_license_checker
cli_helper = _cli_helper
setup_starpls = _setup_starpls
score_coverage_scope = _score_coverage_scope
score_coverage_reporter = _score_coverage_reporter
trlc_requirements_ai_test = _trlc_requirements_ai_test
architecture_ai_test = _architecture_ai_test
