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
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@score_python_basics//:defs.bzl", "score_virtualenv", "score_py_pytest")
load("@pip//:requirements.bzl", "all_requirements")

compile_pip_requirements(
    name = "requirements",
    src = "requirements.in",
    requirements_txt = "requirements.txt",
    tags = [
        "manual",
    ],
)

score_virtualenv(
    venv_name=".rules_ghql_venv",
    reqs=all_requirements,
)

py_binary(
    name = "update_schema",
    srcs = ["update_schema.py"],
)
