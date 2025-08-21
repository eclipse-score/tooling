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

# --- cr_checker ---
load("//cr_checker:cr_checker.bzl", _copyright_checker = "copyright_checker")

# --- dash ---
load("//dash:dash.bzl", _dash_license_checker = "dash_license_checker")

# --- format_checker ---
load("//format_checker:macros.bzl", _use_format_targets = "use_format_targets")

# --- python_basics ---
load(
    "//python_basics:defs.bzl",
    _score_py_pytest = "score_py_pytest",
    _score_virtualenv = "score_virtualenv",
)

# --- starpls ---
load("//starpls:starpls.bzl", _setup_starpls = "setup_starpls")

score_virtualenv = _score_virtualenv
score_py_pytest = _score_py_pytest
dash_license_checker = _dash_license_checker
copyright_checker = _copyright_checker
cli_helper = _cli_helper
use_format_targets = _use_format_targets
setup_starpls = _setup_starpls
