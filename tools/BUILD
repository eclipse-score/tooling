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

load("@rules_shell//shell:sh_binary.bzl", "sh_binary")

package(default_visibility = ["//visibility:public"])

exports_files([
    "actionlint.lock.json",
])

# ruff and yamlfmt are provided by aspect_rules_lint's own bundled
# multitool hub (the default "@multitool" hub). score_tooling doesn't
# re-expose them here: `aspect_rules_lint` is a `dev_dependency` of
# score_tooling, so its `@multitool` hub entries aren't visible to repos that
# merely depend on score_tooling -- consumers should add their own
# `bazel_dep(name = "aspect_rules_lint", ...)` and use
# `@aspect_rules_lint//lint:ruff_bin`, `@aspect_rules_lint//format:yamlfmt`,
# etc. directly. score_tooling only keeps its own dedicated hub for
# actionlint, which aspect_rules_lint doesn't bundle.
sh_binary(
    name = "actionlint",
    srcs = ["@actionlint_hub//tools/actionlint:cwd"],
    visibility = ["//visibility:public"],
)
