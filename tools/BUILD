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

package(default_visibility = ["//visibility:public"])

exports_files([
    "ruff.lock.json",
    "actionlint.lock.json",
    "shellcheck.lock.json",
    "yamlfmt.lock.json",
])

sh_binary(
    name = "ruff",
    srcs = ["@ruff_hub//tools/ruff:cwd"],
    visibility = ["//visibility:public"],
)

sh_binary(
    name = "actionlint",
    srcs = ["@actionlint_hub//tools/actionlint:cwd"],
    visibility = ["//visibility:public"],
)

sh_binary(
    name = "shellcheck",
    srcs = ["@shellcheck_hub//tools/shellcheck:cwd"],
    visibility = ["//visibility:public"],
)

sh_binary(
    name = "yamlfmt",
    srcs = ["@yamlfmt_hub//tools/yamlfmt:cwd"],
    visibility = ["//visibility:public"],
)
