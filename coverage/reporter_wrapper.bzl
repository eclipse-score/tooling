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

"""Executable wrapper rule for the coverage reporter.

Instantiated in the CONSUMER repository (via the score_coverage_reporter
macro) so that the consumer's coverage_scope, MODULE.bazel and LLVM tool
labels can be wired into the reporter, which itself lives in score_tooling.
"""

def _rlocation_path(ctx, file):
    """Return the Runfiles.Rlocation()-compatible path for a Bazel File.

    External-repo files have short_path = "../<repo>/<path>" — strip the
    "../". Main-workspace files have short_path = "<pkg>/<file>" — prepend
    the workspace name. Required because this rule mixes files from the
    consumer repo (_main) and from score_tooling / toolchain repos.
    """
    if file.short_path.startswith("../"):
        return file.short_path[3:]
    return ctx.workspace_name + "/" + file.short_path

def _reporter_wrapper_impl(ctx):
    launcher = ctx.actions.declare_file(ctx.label.name + ".sh")

    reporter = ctx.executable.reporter
    module_bazel = ctx.file.module_bazel
    coverage_scope = ctx.attr.coverage_scope
    allowlist_group = coverage_scope[OutputGroupInfo].allowlist.to_list()
    objects_group = coverage_scope[OutputGroupInfo].objects.to_list()
    object_files = coverage_scope[OutputGroupInfo].object_files

    if len(allowlist_group) != 1:
        fail("coverage_scope must provide exactly one allowlist file")
    if len(objects_group) != 1:
        fail("coverage_scope must provide exactly one objects manifest file")

    allowlist = allowlist_group[0]
    baseline_objects = objects_group[0]

    cxxfilt_line = ""
    if ctx.file.llvm_cxxfilt:
        cxxfilt_line = '  --llvm_cxxfilt="{}" \\\n'.format(
            _rlocation_path(ctx, ctx.file.llvm_cxxfilt),
        )

    # Bazel invokes the coverage report generator from within its coverage
    # machinery, where an inherited RUNFILES_DIR may point at ANOTHER tool's
    # runfiles tree. Derive our own runfiles directory from $0 first and only
    # fall back to the inherited value.
    script = """#!/usr/bin/env bash
set -euo pipefail
SELF_RUNFILES_DIR="$(cd "$(dirname "$0")" && pwd)/$(basename "$0").runfiles"
if [[ -d "${{SELF_RUNFILES_DIR}}" ]]; then
  RUNFILES_DIR="${{SELF_RUNFILES_DIR}}"
elif [[ -z "${{RUNFILES_DIR:-}}" || ! -d "${{RUNFILES_DIR}}" ]]; then
  echo "ERROR: could not locate the reporter_wrapper runfiles directory" >&2
  exit 1
fi
export RUNFILES_DIR
WORKSPACE_ROOT="$(cd "$(dirname "$(readlink -f "${{RUNFILES_DIR}}/{module_bazel}")")" && pwd)/"
exec "${{RUNFILES_DIR}}/{reporter}" \\
  --coverage_allowlist="{allowlist}" \\
  --baseline_objects="{baseline_objects}" \\
  --workspace_root="${{WORKSPACE_ROOT}}" \\
  --llvm_cov="{llvm_cov}" \\
  --llvm_profdata="{llvm_profdata}" \\
{cxxfilt_line}  "$@"
""".format(
        module_bazel = _rlocation_path(ctx, module_bazel),
        reporter = _rlocation_path(ctx, reporter),
        allowlist = _rlocation_path(ctx, allowlist),
        baseline_objects = _rlocation_path(ctx, baseline_objects),
        llvm_cov = _rlocation_path(ctx, ctx.file.llvm_cov),
        llvm_profdata = _rlocation_path(ctx, ctx.file.llvm_profdata),
        cxxfilt_line = cxxfilt_line,
    )

    ctx.actions.write(
        output = launcher,
        content = script,
        is_executable = True,
    )

    direct_files = [
        reporter,
        allowlist,
        baseline_objects,
        module_bazel,
        ctx.file.llvm_cov,
        ctx.file.llvm_profdata,
    ]
    if ctx.file.llvm_cxxfilt:
        direct_files.append(ctx.file.llvm_cxxfilt)

    runfiles = ctx.runfiles(
        files = direct_files,
        transitive_files = object_files,
    ).merge(ctx.attr.reporter[DefaultInfo].default_runfiles)
    for tool in (ctx.attr.llvm_cov, ctx.attr.llvm_profdata, ctx.attr.llvm_cxxfilt):
        if tool:
            runfiles = runfiles.merge(tool[DefaultInfo].default_runfiles)

    return [DefaultInfo(
        executable = launcher,
        runfiles = runfiles,
    )]

reporter_wrapper = rule(
    implementation = _reporter_wrapper_impl,
    executable = True,
    attrs = {
        "reporter": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//coverage:reporter"),
            doc = "The coverage reporter binary (defaults to score_tooling's).",
        ),
        "coverage_scope": attr.label(
            cfg = "target",
            mandatory = True,
            doc = "A score_coverage_scope target defining the in-scope sources/archives.",
        ),
        "module_bazel": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "The consumer's root MODULE.bazel; used to locate the real workspace root.",
        ),
        "llvm_cov": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "llvm-cov binary, e.g. @llvm_toolchain//:llvm-cov.",
        ),
        "llvm_profdata": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "llvm-profdata binary, e.g. @llvm_toolchain//:llvm-profdata.",
        ),
        "llvm_cxxfilt": attr.label(
            allow_single_file = True,
            doc = "Optional llvm-cxxfilt for demangling, e.g. " +
                  "@llvm_toolchain_llvm//:bin/llvm-cxxfilt.",
        ),
    },
)
