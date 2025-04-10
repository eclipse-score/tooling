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
load("@aspect_rules_py//py:defs.bzl", "py_venv")
load("@pip_score_python_basics//:requirements.bzl", "all_requirements")


def score_virtualenv(name = "ide_support", venv_name =".venv",  reqs = []):
    py_venv(
        name = name,
        venv_name = venv_name,
        deps = all_requirements + reqs
        data = ["//pyproject.toml"]
    )
