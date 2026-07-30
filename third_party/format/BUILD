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

load("@rules_shell//shell:sh_binary.bzl", "sh_binary")

package(default_visibility = ["//visibility:public"])

sh_binary(
    name = "rustfmt_with_policies",
    srcs = ["tool/rustfmt_with_policies.sh"],
    data = [
        "@bazel_tools//tools/bash/runfiles",
        "@rules_rust//tools/upstream_wrapper:rustfmt",
        "@score_rust_policies//rustfmt:rustfmt.toml",
    ],
)
