# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
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

load("@rules_python//python:defs.bzl", "py_test")

py_test(
    name = "merger_test",
    srcs = ["merger_test.py"],
    deps = ["//coverage:merger_lib"],
)

py_test(
    name = "reporter_test",
    srcs = ["reporter_test.py"],
    deps = ["//coverage:reporter_lib"],
)
