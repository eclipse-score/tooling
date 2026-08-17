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

"""Public API of the score_tooling LLVM source-based coverage pipeline.

Consumers instantiate two targets in their own repository (typically in a
tools/coverage/BUILD or quality/coverage/BUILD file):

    load("@score_tooling//coverage:defs.bzl",
         "score_coverage_reporter", "score_coverage_scope")

    score_coverage_scope(
        name = "coverage_scope",
        testonly = True,
        deps = ["//src/mylib", "//src/rust/mycrate"],
    )

    score_coverage_reporter(
        name = "reporter_wrapper",
        testonly = True,
        coverage_scope = ":coverage_scope",
        llvm_cov = "@llvm_toolchain//:llvm-cov",
        llvm_profdata = "@llvm_toolchain//:llvm-profdata",
        llvm_cxxfilt = "@llvm_toolchain_llvm//:bin/llvm-cxxfilt",
    )

and point Bazel at them from their coverage bazelrc config:

    coverage:llvm_cov --coverage_output_generator=@score_tooling//coverage:merger
    coverage:llvm_cov --coverage_report_generator=//tools/coverage:reporter_wrapper

See coverage/README.md for the complete adoption guide (toolchains, bazelrc,
justifications, CI) and coverage/COVERAGE_GUIDE.md for how the pipeline works.
"""

load("//coverage:coverage_scope.bzl", _coverage_scope = "coverage_scope")
load("//coverage:reporter_wrapper.bzl", _reporter_wrapper = "reporter_wrapper")

score_coverage_scope = _coverage_scope

def score_coverage_reporter(
        name,
        coverage_scope,
        llvm_cov,
        llvm_profdata,
        llvm_cxxfilt = None,
        module_bazel = "//:MODULE.bazel",
        **kwargs):
    """Declare the consumer-side coverage report generator.

    The generated executable is passed to Bazel as
    --coverage_report_generator=//<pkg>:<name>. It wires the consumer's
    coverage scope, workspace root and LLVM tools into score_tooling's
    reporter.

    Args:
        name: Target name, referenced by --coverage_report_generator.
        coverage_scope: A score_coverage_scope target listing the production
            targets that define the coverage scope.
        llvm_cov: Label of the llvm-cov binary (the consumer's LLVM toolchain,
            e.g. "@llvm_toolchain//:llvm-cov"). Must come from the same LLVM
            major version that produced the coverage instrumentation.
        llvm_profdata: Label of the llvm-profdata binary.
        llvm_cxxfilt: Optional label of llvm-cxxfilt for symbol demangling
            (C++ Itanium and Rust v0/legacy). toolchains_llvm exposes it as
            "@llvm_toolchain_llvm//:bin/llvm-cxxfilt".
        module_bazel: The consumer's root MODULE.bazel, used at runtime to
            locate the real workspace root. Requires
            exports_files(["MODULE.bazel"]) in the consumer's root BUILD file.
        **kwargs: Common rule attributes (testonly, visibility, tags, ...).
    """
    _reporter_wrapper(
        name = name,
        coverage_scope = coverage_scope,
        module_bazel = module_bazel,
        llvm_cov = llvm_cov,
        llvm_profdata = llvm_profdata,
        llvm_cxxfilt = llvm_cxxfilt,
        **kwargs
    )
