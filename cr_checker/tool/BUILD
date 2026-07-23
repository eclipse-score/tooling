# *******************************************************************************
# Copyright (c) 2024 Contributors to the Eclipse Foundation
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
load("@aspect_rules_py//py:defs.bzl", "py_library")
load("@pip_cr_checker//:requirements.bzl", "requirement")

py_library(
    name = "cr_checker_lib",
    srcs = [
        "cr_checker.py",
    ],
    visibility = ["//visibility:public"],
    deps = [
        requirement("rapidfuzz"),
    ],
)
