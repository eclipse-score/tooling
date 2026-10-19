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

# Sub-package fixture: a fault_trees target without `fta_package` gets a
# default package name that contains the Bazel package path.

load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "fault_trees")

package(default_visibility = ["//visibility:public"])

fault_trees(
    name = "default_package_fault_trees",
)
