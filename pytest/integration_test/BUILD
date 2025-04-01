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
load("@score_pytest//:pytest.bzl", "score_py_pytest")

score_py_pytest(
    name = "pytest_version_correct",
    srcs = [
        "int_test.py",
    ],
)   

compile_pip_requirements(
    name = "requirements",
    srcs = ["requirements.txt","@score_pytest//:requirements_lock.txt"],
    requirements_txt = "requirements_lock.txt",
    tags = [
        "manual",
    ],
)
