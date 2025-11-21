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
load("@aspect_rules_lint//format:defs.bzl", "format_multirun", "format_test")

def use_format_targets(fix_name = "format.fix", check_name = "format.check"):
    format_multirun(
        name = fix_name,
        python = "@aspect_rules_lint//format:ruff",
        rust = "@rules_rust//tools/upstream_wrapper:rustfmt",
        starlark = "@buildifier_prebuilt//:buildifier",
        yaml = "@aspect_rules_lint//format:yamlfmt",
        visibility = ["//visibility:public"],
    )

    format_test(
        name = check_name,
        no_sandbox = True,
        python = "@aspect_rules_lint//format:ruff",
        rust = "@rules_rust//tools/upstream_wrapper:rustfmt",
        starlark = "@buildifier_prebuilt//:buildifier",
        yaml = "@aspect_rules_lint//format:yamlfmt",
        workspace = "//:MODULE.bazel",
        visibility = ["//visibility:public"],
    )
