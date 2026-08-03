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

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "integration_lib",
    srcs = ["lib.rs"],
    edition = "2021",
)

rust_test(
    name = "integration_lib_test",
    crate = ":integration_lib",
    edition = "2021",
)

# No test runs this binary — its main.rs must show up at exact 0% via the
# --empty-profile baseline over the coverage-built executable.
rust_binary(
    name = "untested_tool",
    srcs = ["main.rs"],
    edition = "2021",
    deps = [":integration_lib"],
)
