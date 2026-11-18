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
"""Traces tests to the tool requirements.

A test traces to tool requirements via tags `lobster-tracing=<id>`.
"""

load("@lobster//:lobster.bzl", "lobster_gtest")
load("@score_tooling//bazel/rules/rules_score/docs/requirements:traceability.bzl", "tool_traceability_test")

visibility("private")

_TRACE_PREFIX = "lobster-tracing="

_CaseInfo = provider(
    doc = "Requirement references, location and, for analysis tests, the result of one test.",
    fields = ["label", "file", "executed", "success", "message", "refs"],
)

def _refs(tags):
    return [t[len(_TRACE_PREFIX):] for t in tags if t.startswith(_TRACE_PREFIX)]

def _location(target, ctx):
    srcs = getattr(ctx.rule.files, "srcs", [])
    if len(srcs) == 1 and srcs[0].is_source:
        return srcs[0].path

    # Assumes the package file is named BUILD.
    label = target.label
    return "/".join([p for p in [label.workspace_root, label.package, "BUILD"] if p])

def _case_aspect_impl(target, ctx):
    executed = AnalysisTestResultInfo in target
    result = target[AnalysisTestResultInfo] if executed else None
    return [_CaseInfo(
        label = target.label,
        file = _location(target, ctx),
        executed = executed,
        success = result.success if executed else True,
        message = result.message if executed else "",
        refs = _refs(ctx.rule.attr.tags),
    )]

_case_aspect = aspect(
    implementation = _case_aspect_impl,
    doc = "Collects the requirement references of a test.",
)

def _test_id(label):
    # No "@": lobster reads it as the tag version separator.
    return "{}//{}:{}".format(label.repo_name, label.package, label.name)

def _tool_test_report_impl(ctx):
    spec = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(spec, json.encode([
        {
            "name": _test_id(case.label),
            "file": case.file,
            "refs": case.refs,
            "executed": case.executed,
            "failure": None if case.success else case.message,
        }
        for case in [t[_CaseInfo] for t in ctx.attr.tests]
    ]))
    xml = ctx.actions.declare_file(ctx.label.name + ".xml")
    ctx.actions.run(
        executable = ctx.executable._xml_writer,
        arguments = ["--spec", spec.path, "--out", xml.path],
        inputs = [spec],
        outputs = [xml],
        mnemonic = "ToolTestsXml",
    )
    return [DefaultInfo(files = depset([xml]))]

_tool_test_report = rule(
    implementation = _tool_test_report_impl,
    attrs = {
        "tests": attr.label_list(
            doc = """Tests to report. Analysis tests report their result;
            other tests are reported as not run, as their result is only known
            when they run.""",
            aspects = [_case_aspect],
            mandatory = True,
        ),
        "_xml_writer": attr.label(
            default = Label("//:tool_tests_xml"),
            executable = True,
            cfg = "exec",
        ),
    },
    doc = "Writes the requirement references of tests as gtest XML.",
)

def _is_traced(rule):
    return len(_refs(rule.get("tags", []))) > 0

def tool_traceability(name = "tool_traceability_test", tests = [], tags = []):
    """Checks that the tool requirements trace to tests.

    Call after all tests of the package are defined. Traces every test of the
    package tagged `lobster-tracing=...` plus `tests` of other packages.

    Defines `tool_tests_suite` running all traced tests and `name` checking the
    traceability of the tool requirements.

    Args:
        name: Name of the traceability check.
        tests: Traced tests of other packages.
        tags: Tags of the traceability check.
    """
    traced = sorted([
        ":" + n
        for n, rule in native.existing_rules().items()
        if _is_traced(rule)
    ]) + tests
    _tool_test_report(
        name = "tool_tests_report",
        testonly = True,
        tests = traced,
    )
    lobster_gtest(
        name = "tool_tests",
        testonly = True,
        tests = [":tool_tests_report"],
    )
    native.test_suite(
        name = "tool_tests_suite",
        tests = traced,
    )
    tool_traceability_test(
        name = name,
        tags = tags,
        tool_tests = [":tool_tests"],
    )
