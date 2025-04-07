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
load("@bazel_skylib//rules:build_test.bzl", "build_test")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@score_python_basics//score_venv:py_pyvenv.bzl", "score_virtualenv")
load("@score_python_basics//score_pytest:py_pytest.bzl", "score_py_pytest")
load("@pip_score_venv_test//:requirements.bzl", "all_requirements")

compile_pip_requirements(
    name = "requirements",
    src = "requirements.in",
    requirements_txt = "requirements.txt",
    tags = [
        "manual",
    ],
)


build_test(
    name = "venv_functions",
    targets = [
        ":basic_venv"
    ],
)

score_virtualenv(
    name="basic_venv",
    venv_name=".basic_venv"
)

score_py_pytest(
    name = "venv_ok_test",
    srcs = [ "test_venv_ok.py"],
    data = [":basic_venv"]
)

score_virtualenv(
    name="additional_req_venv",
    venv_name=".additional_req_venv",
    reqs=all_requirements,
)

score_py_pytest(
    name = "venv_accepts_outside_reqs",
    srcs = [ "test_extra_requirements_ok.py"],
    data = [":additional_req_venv"]
)
