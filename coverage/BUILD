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

load("@pip_coverage//:requirements.bzl", "requirement")
load("@rules_cc//cc/toolchains:args.bzl", "cc_args")
load("@rules_cc//cc/toolchains:feature.bzl", "cc_feature")
load("@rules_python//python:defs.bzl", "py_binary", "py_library")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@rules_shell//shell:sh_binary.bzl", "sh_binary")

package(default_visibility = ["//visibility:public"])

# =============================================================================
# LLVM source-based coverage pipeline (C++ + Rust).
# Consumer-facing API lives in defs.bzl; see README.md for the adoption guide
# and COVERAGE_GUIDE.md for how the pipeline works.
# =============================================================================

# Per-test coverage output generator. Referenced by consumers directly:
#   coverage:llvm_cov --coverage_output_generator=@score_tooling//coverage:merger
# It locates llvm-profdata via the LLVM_PROFDATA / RUST_LLVM_PROFDATA
# environment variables exported by the toolchains, so it needs no
# consumer-supplied labels.
py_binary(
    name = "merger",
    srcs = ["merger.py"],
)

# Final report generator. NOT referenced directly by consumers — the
# score_coverage_reporter macro wraps it with the consumer's coverage scope,
# workspace root and LLVM tool labels.
py_binary(
    name = "reporter",
    srcs = ["reporter.py"],
    deps = ["@rules_python//python/runfiles"],
)

py_binary(
    name = "justify",
    srcs = ["justify.py"],
    deps = [requirement("pyyaml")],
)

py_binary(
    name = "effective_coverage",
    srcs = ["effective_coverage.py"],
)

# Markdown job summary (GITHUB_STEP_SUMMARY / --summary-md), invoked by
# generate_coverage_html.sh. Stdlib only.
py_binary(
    name = "coverage_summary",
    srcs = ["coverage_summary.py"],
)

sh_binary(
    name = "generate_coverage_html",
    srcs = ["generate_coverage_html.sh"],
)

# Import-only libraries so unit tests can `from coverage.merger import ...`.
py_library(
    name = "merger_lib",
    srcs = ["merger.py"],
    imports = [".."],
)

py_library(
    name = "reporter_lib",
    srcs = ["reporter.py"],
    imports = [".."],
    deps = ["@rules_python//python/runfiles"],
)

# Import-only test helper like merger_lib/reporter_lib above: imports=[".."]
# makes `from coverage.coverage_summary import ...` resolvable from the unit
# tests. Only the *_lib targets carry it; the binaries are unaffected.
py_library(
    name = "coverage_summary_lib",
    srcs = ["coverage_summary.py"],
    imports = [".."],
)

# These compile time options are required to cover abnormal termination cases
# (death tests). LLVM provides them in combination with a specific profile
# setting which is enabled in Bazel via LLVM_PROFILE_CONTINUOUS_MODE.
# Consumers reference this feature from their toolchains_llvm extension config:
#   llvm.toolchain(extra_known_features =
#       ["@score_tooling//coverage:enable_llvm_coverage_for_death_tests"], ...)
cc_args(
    name = "runtime_relocation_args",
    actions = [
        "@rules_cc//cc/toolchains/actions:compile_actions",
        "@rules_cc//cc/toolchains/actions:link_actions",
    ],
    args = [
        "-mllvm",
        "-runtime-counter-relocation",
    ],
)

cc_feature(
    name = "enable_llvm_coverage_for_death_tests",
    args = [":runtime_relocation_args"],
    feature_name = "enable_llvm_coverage_for_death_tests",
)

# In order to update the requirements, change the `requirements.in` file and run:
# `bazel run //coverage:requirements_3_XX.update  --@@rules_python+//python/config_settings:python_version=3.XX`.
[
    compile_pip_requirements(
        name = "requirements_3_{}".format(version),
        src = "requirements.in",
        python_version = "3.{}".format(version),
        requirements_txt = "requirements_3_{}.txt".format(version),
        tags = [
            "manual",
        ],
    )
    for version in [
        "11",
        "12",
    ]
]

# =============================================================================
# Legacy helpers (kept):
#   - llvm_profile_wrapper: still referenced by downstream ferrocene-coverage
#     configs (test --run_under=...) during migration to the LLVM pipeline.
#   - combined_report: Rust + Python lcov/genhtml report for THIS repository's
#     own tools, wired into deploy_docs.yml.
# =============================================================================

sh_binary(
    name = "llvm_profile_wrapper",
    srcs = ["llvm_profile_wrapper.sh"],
    visibility = ["//visibility:public"],
)

sh_binary(
    name = "combined_report",
    srcs = ["run_combined_coverage.sh"],
    data = ["@lcov_deb//:srcs"],
)
