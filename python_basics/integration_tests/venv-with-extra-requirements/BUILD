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
load("@pip_score_venv_test//:requirements.bzl", "all_requirements")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@score_tooling//python_basics:defs.bzl", "score_py_pytest", "score_virtualenv")

compile_pip_requirements(
    name = "requirements",
    src = "requirements.in",
    requirements_txt = "requirements.txt",
    tags = [
        "manual",
    ],
)

score_virtualenv(
    name = "additional_req_venv",
    reqs = all_requirements,
    venv_name = ".additional_req_venv",
)

score_py_pytest(
    name = "venv_accepts_outside_reqs",
    srcs = ["test_extra_requirements_ok.py"],
    data = [":additional_req_venv"],
    deps = all_requirements,
)
