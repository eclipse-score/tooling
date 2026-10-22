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
"""Runs lobster-ci-report on the traceability report of a dependable_element index and checks its output."""

load("@bazel_skylib//lib:shell.bzl", "shell")
load("@score_tooling//bazel/rules/rules_score:providers.bzl", "DependableElementLobsterInfo")

def _lobster_ci_report_test_impl(ctx):
    report = ctx.attr.index[DependableElementLobsterInfo].lobster_report
    ci = ctx.executable._lobster_ci_report

    lines = [
        "out=$TEST_TMPDIR/lobster-ci-report.out",
        "{} {} > \"$out\" 2>&1".format(shell.quote(ci.short_path), shell.quote(report.short_path)),
        "rc=$?",
    ]
    if ctx.attr.expect_ci_failure:
        lines.append("[ \"$rc\" -ne 0 ] || { echo 'lobster-ci-report passed, a failure was expected'; cat \"$out\"; exit 1; }")
    for text in ctx.attr.contains:
        lines.append("grep -qF -- {t} \"$out\" || {{ echo {m}; cat \"$out\"; exit 1; }}".format(
            t = shell.quote(text),
            m = shell.quote("missing in the lobster-ci-report output: " + text),
        ))
    for text in ctx.attr.lacks:
        lines.append("! grep -qF -- {t} \"$out\" || {{ echo {m}; cat \"$out\"; exit 1; }}".format(
            t = shell.quote(text),
            m = shell.quote("unexpected in the lobster-ci-report output: " + text),
        ))

    script = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.write(script, "\n".join(lines) + "\n", is_executable = True)

    runfiles = ctx.runfiles(files = [ci, report]).merge(ctx.attr._lobster_ci_report[DefaultInfo].default_runfiles)
    return [DefaultInfo(executable = script, runfiles = runfiles)]

lobster_ci_report_test = rule(
    implementation = _lobster_ci_report_test_impl,
    doc = "Runs lobster-ci-report on the report of a dependable_element index; fails unless the exit status and output match.",
    attrs = {
        "index": attr.label(
            mandatory = True,
            providers = [DependableElementLobsterInfo],
            doc = "The `<name>_index` target of a dependable_element.",
        ),
        "expect_ci_failure": attr.bool(
            default = False,
            doc = "Require a non-zero exit status.",
        ),
        "contains": attr.string_list(
            doc = "Texts the output must contain.",
        ),
        "lacks": attr.string_list(
            doc = "Texts the output must not contain.",
        ),
        "_lobster_ci_report": attr.label(
            default = Label("@lobster//:lobster-ci-report"),
            executable = True,
            cfg = "exec",
        ),
    },
    test = True,
)
