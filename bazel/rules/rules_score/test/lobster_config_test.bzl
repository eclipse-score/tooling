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
"""
Loading-phase unit tests for format_lobster_sources()/format_lobster_block()
(bazel/rules/rules_score/private/lobster_config.bzl).

Both functions are pure Starlark -- they only read `.path` off whatever is
passed as a "file" -- so plain `struct(path = ...)` fakes stand in for real
File objects, and skylib's `loadingtest` (not `analysistest`) is used: no
target_under_test / analysis phase is needed to exercise them.
"""

load("@bazel_skylib//lib:unittest.bzl", "loadingtest")
load(
    "@score_tooling//bazel/rules/rules_score/private:lobster_config.bzl",
    "format_lobster_block",
    "format_lobster_sources",
)

def _fake_file(path):
    return struct(path = path)

def lobster_config_test_suite(name):
    """Defines the loading-phase test suite for lobster_config.bzl helpers.

    Args:
        name: Suite name; individual test targets and the aggregating
            `<name>_tests` test_suite are derived from it.
    """
    env = loadingtest.make(name)

    # --- format_lobster_sources ----------------------------------------------

    loadingtest.equals(
        env,
        "sources_empty",
        "",
        format_lobster_sources([]),
    )

    loadingtest.equals(
        env,
        "sources_multiple",
        '  source: "a.lobster";\n  source: "b.lobster";',
        format_lobster_sources([_fake_file("a.lobster"), _fake_file("b.lobster")]),
    )

    # --- format_lobster_block: empty level is omitted by default -------------
    # This is the core bug fix: an empty level must not appear at all (and
    # thus carry no `trace to:` edge), instead of rendering an empty block
    # that would make LOBSTER flag every item at the target level as missing
    # a reference.

    loadingtest.equals(
        env,
        "block_empty_omitted",
        "",
        format_lobster_block("requirements", "Feature Requirements", []),
    )

    loadingtest.equals(
        env,
        "block_empty_omitted_even_with_trace_to",
        "",
        format_lobster_block(
            "requirements",
            "Component Requirements",
            [],
            trace_to = ["Feature Requirements"],
        ),
    )

    # --- format_lobster_block: emit_empty keeps the trace-to edge active -----
    # Used by callers in strict/release mode to force a checking level (e.g.
    # Unit Test) to exist even without sources, so LOBSTER still reports
    # missing coverage of the target level instead of silently skipping it.

    loadingtest.equals(
        env,
        "block_empty_emit_empty_keeps_trace_to",
        'activity "Unit Test" {\n\n  trace to: "Component Requirements";\n}',
        format_lobster_block(
            "activity",
            "Unit Test",
            [],
            trace_to = ["Component Requirements"],
            emit_empty = True,
        ),
    )

    loadingtest.equals(
        env,
        "block_empty_emit_empty_no_trace_to",
        'implementation "Public API" {\n\n}',
        format_lobster_block(
            "implementation",
            "Public API",
            [],
            emit_empty = True,
        ),
    )

    # --- format_lobster_block: non-empty level ---------------------------------

    loadingtest.equals(
        env,
        "block_with_sources_no_trace",
        'requirements "Feature Requirements" {\n  source: "feat.lobster";\n}',
        format_lobster_block("requirements", "Feature Requirements", [_fake_file("feat.lobster")]),
    )

    # --- format_lobster_block: non-empty level, guarded trace_to present -------
    # This is the "guarded" half of the fix: callers only pass a `trace_to`
    # name when that target level is itself non-empty in the same config;
    # this case verifies the trace line renders correctly once a caller does
    # pass one.

    loadingtest.equals(
        env,
        "block_with_sources_and_guarded_trace_to",
        'requirements "Component Requirements" {\n  source: "comp.lobster";\n  trace to: "Feature Requirements";\n}',
        format_lobster_block(
            "requirements",
            "Component Requirements",
            [_fake_file("comp.lobster")],
            trace_to = ["Feature Requirements"],
        ),
    )

    # --- format_lobster_block: multiple trace_to targets -----------------------

    loadingtest.equals(
        env,
        "block_with_multiple_trace_to",
        'activity "Root Causes" {\n  source: "rc.lobster";\n  trace to: "Failure Modes";\n  trace to: "Control Measures";\n}',
        format_lobster_block(
            "activity",
            "Root Causes",
            [_fake_file("rc.lobster")],
            trace_to = ["Failure Modes", "Control Measures"],
        ),
    )

    # --- format_lobster_block: requires (OR-of-sources override) --------------
    # By default, when multiple different levels each `trace to:` the same
    # target, LOBSTER requires ALL of them to independently cover every item
    # (AND across sources). `requires` overrides this to "any one of these is
    # sufficient" for a given target level -- see e.g. Received AoUs, which
    # must be covered by EITHER Component Requirements OR Forwarded AoUs, not
    # both at once.

    loadingtest.equals(
        env,
        "block_with_requires_or_group",
        'requirements "Received AoUs" {\n  source: "aou.lobster";\n  requires: "Component Requirements" or "Forwarded AoUs";\n}',
        format_lobster_block(
            "requirements",
            "Received AoUs",
            [_fake_file("aou.lobster")],
            requires = [["Component Requirements", "Forwarded AoUs"]],
        ),
    )

    loadingtest.equals(
        env,
        "block_with_requires_single_name",
        'requirements "Received AoUs" {\n  source: "aou.lobster";\n  requires: "Component Requirements";\n}',
        format_lobster_block(
            "requirements",
            "Received AoUs",
            [_fake_file("aou.lobster")],
            requires = [["Component Requirements"]],
        ),
    )

    loadingtest.equals(
        env,
        "block_with_requires_empty_group_omitted",
        'requirements "Received AoUs" {\n  source: "aou.lobster";\n}',
        format_lobster_block(
            "requirements",
            "Received AoUs",
            [_fake_file("aou.lobster")],
            requires = [[]],
        ),
    )

    loadingtest.equals(
        env,
        "block_with_trace_to_and_requires_combined",
        'requirements "Component Requirements" {\n  source: "comp.lobster";\n  trace to: "Feature Requirements";\n  requires: "Feature Requirements" or "Received AoUs";\n}',
        format_lobster_block(
            "requirements",
            "Component Requirements",
            [_fake_file("comp.lobster")],
            trace_to = ["Feature Requirements"],
            requires = [["Feature Requirements", "Received AoUs"]],
        ),
    )
