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

load("@bazel_skylib//rules:diff_test.bzl", "diff_test")
load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load("//:l4.bzl", "l4_test")

sh_binary(
    name = "run_child",
    srcs = ["run_child.sh"],
)

l4_test(
    name = "minimal_example_test",
    module = "minimal_requirements",
)

l4_test(
    name = "integrator_example_test",
    module = "integrator",
    overrides = [
        "seooc",
        "some_other_library",
    ],
    variant_build = [
        "@some_other_library//:other_seooc",
        "@some_other_library//:some_other_library",
        "--@some_other_library//:use_variant_2=true",
    ],
)

l4_test(
    name = "failure_cases_test",
    cases = "@failure_cases//:cases.tsv",
    module = "failure_cases",
)

# Outer Bazel of this module and the nested Bazel (bazel_binaries) must match the root workspace.
diff_test(
    name = "bazelversion_in_sync_test",
    file1 = ".bazelversion",
    file2 = "@score_tooling//:.bazelversion",
)
