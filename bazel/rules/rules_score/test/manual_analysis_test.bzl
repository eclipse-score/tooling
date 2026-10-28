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

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@lobster//:lobster.bzl", "LobsterProvider")

def _manual_evidence_impl(ctx):
    output = ctx.actions.declare_file(ctx.label.name + ctx.attr.extension)
    ctx.actions.write(output, '{"schema":"lobster-act-trace","version":3,"generator":"test","data":[]}')
    unrelated = ctx.actions.declare_file(ctx.label.name + "_unrelated.lobster")
    ctx.actions.write(unrelated, "not a LOBSTER input")
    return [
        DefaultInfo(
            files = depset([unrelated] + ([] if ctx.attr.runfiles_only else [output])),
            runfiles = ctx.runfiles(files = [output]),
        ),
        LobsterProvider(lobster_input = {output.basename: output.path}),
    ]

manual_evidence = rule(
    implementation = _manual_evidence_impl,
    attrs = {
        "extension": attr.string(default = ".lobster"),
        "runfiles_only": attr.bool(default = False),
    },
)

def _manual_analysis_report_test_impl(ctx):
    env = analysistest.begin(ctx)
    actions = analysistest.target_actions(env)
    configs = [
        action
        for action in actions
        if any([output.basename == "de_traceability_config" for output in action.outputs.to_list()])
    ]
    asserts.equals(env, 1, len(configs))
    substitutions = configs[0].substitutions
    manual_block = substitutions["{MANUAL_ANALYSIS_BLOCK}"]
    component_block = substitutions["{COMP_REQ_BLOCK}"]
    feature_block = substitutions["{FEAT_REQ_BLOCK}"]
    if ctx.attr.expect_manual_analysis:
        asserts.true(env, 'activity "Manual Analysis"' in manual_block)
        asserts.true(env, "manual_evidence.lobster" in manual_block)
        asserts.true(env, 'trace to: "Component Requirements";' in manual_block)
        asserts.true(env, 'trace to: "Feature Requirements";' in manual_block)
        asserts.true(env, 'requires: "Unit Test" or "Manual Analysis";' in component_block)
        asserts.false(env, 'requires: "Unit Test";' in component_block)
        asserts.true(env, 'requires: "Architecture";' in component_block)
        asserts.false(env, 'requires: "Architecture" or "Manual Analysis";' in component_block)
        asserts.equals(
            env,
            ctx.attr.expect_coverage,
            'requires: "Test Case Coverage" or "Manual Analysis";' in component_block,
        )
        asserts.false(env, 'requires: "Test Case Coverage";' in component_block)
        asserts.true(env, 'requires: "Component Requirements";' in feature_block)
        reports = [
            action
            for action in actions
            if any([output.basename.endswith("_report.json") for output in action.outputs.to_list()])
        ]
        asserts.equals(env, 1, len(reports))
        for evidence_name in ["manual_evidence.lobster"] + ctx.attr.extra_evidence:
            asserts.true(env, evidence_name in manual_block)
            asserts.true(
                env,
                any([file.basename == evidence_name for file in reports[0].inputs.to_list()]),
                "The aggregated report must consume " + evidence_name,
            )
        asserts.false(env, "_unrelated.lobster" in manual_block)
        asserts.true(
            env,
            not any([file.basename.endswith("_unrelated.lobster") for file in reports[0].inputs.to_list()]),
            "The aggregated report must not consume evidence absent from LobsterProvider",
        )
    else:
        asserts.equals(env, "", manual_block)
        asserts.false(env, "requires:" in component_block)
        asserts.false(env, "requires:" in feature_block)
    return analysistest.end(env)

manual_analysis_report_test = analysistest.make(
    _manual_analysis_report_test_impl,
    attrs = {
        "expect_manual_analysis": attr.bool(default = True),
        "expect_coverage": attr.bool(default = False),
        "extra_evidence": attr.string_list(),
    },
)

def _manual_analysis_missing_output_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(env, "must export LOBSTER input")
    asserts.expect_failure(env, "manual_evidence_runfiles.json' in DefaultInfo.files")
    return analysistest.end(env)

manual_analysis_missing_output_test = analysistest.make(
    _manual_analysis_missing_output_test_impl,
    expect_failure = True,
)
