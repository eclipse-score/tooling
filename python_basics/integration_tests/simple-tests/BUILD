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
load("@score_python_basics//:defs.bzl", "score_virtualenv", "score_py_pytest")

score_virtualenv(
    name="basic_venv",
    venv_name=".basic_venv"
)

score_py_pytest(
    name = "venv_ok_test",
    srcs = [ "test_venv_ok.py"],
    data = [":basic_venv"]
)

score_py_pytest(
    name = "pytest_version_correct",
    srcs = [
        "pytest_ok.py",
    ],
)
