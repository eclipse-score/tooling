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
load("@score_tooling//starpls:starpls.bzl", "setup_starpls")
load(":defs.bzl", "starpls_py_integration_test")

setup_starpls(
    name = "_starpls_binary_for_test",
    visibility = ["//visibility:public"],
)

starpls_py_integration_test(
    name = "starpls_test",
    srcs = ["starpls_test.py"],
    data = [":_starpls_binary_for_test_bin"],
)
