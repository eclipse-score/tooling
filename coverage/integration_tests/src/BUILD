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

load("@rules_cc//cc:cc_library.bzl", "cc_library")
load("@rules_cc//cc:cc_test.bzl", "cc_test")

package(default_visibility = ["//visibility:public"])

cc_library(
    name = "coverable",
    srcs = ["coverable.cpp"],
    hdrs = ["coverable.h"],
)

# No test links against this library — it must show up at exact 0% in the
# report via the --empty-profile baseline.
cc_library(
    name = "uncovered",
    srcs = ["uncovered.cpp"],
    hdrs = ["uncovered.h"],
)

cc_test(
    name = "coverable_test",
    srcs = ["coverable_test.cpp"],
    deps = [":coverable"],
)
