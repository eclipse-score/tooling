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
# load("//tools:ruff.bzl", "ruff_binary")
# load("//score_venv/py_pyvenv.bzl", "score_virtualenv")

load("@aspect_rules_py//py:defs.bzl", "py_binary", "py_library", "py_venv")
load("@pip_tooling//:requirements.bzl", "all_requirements")
load("@score_tooling//python_basics/score_pytest:py_pytest.bzl", _score_py_pytest = "score_py_pytest")

# Export score_py_pytest
score_py_pytest = _score_py_pytest

def score_virtualenv(name = "ide_support", venv_name = ".venv", reqs = [], tags = [], data = []):
    py_venv(
        name = name,
        venv_name = venv_name,
        deps = all_requirements + reqs + ["@rules_python//python/runfiles"] ,
        data = ["@score_tooling//python_basics:pyproject.toml"] + data,
        tags = tags,
    )
