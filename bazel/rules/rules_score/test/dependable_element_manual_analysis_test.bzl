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
load("@rules_python//python:defs.bzl", "py_test")
load("@score_tooling//bazel/rules/rules_score:providers.bzl", "DependableElementLobsterInfo")
load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "dependable_element")
load("@score_tooling//manual_analysis:manual_analysis.bzl", "manual_analysis")

def _manual_analysis_inputs_impl(ctx):
    analysis = ctx.actions.declare_file(ctx.label.name + ".yaml")
    results = ctx.actions.declare_file(ctx.label.name + ".json")
    lock = ctx.actions.declare_file(ctx.label.name + ".lock")
    ctx.actions.write(analysis, json.encode({
        "requirements": ["AouRootCauseAous.ConfigurationIsCorrect@1"],
        "steps": [{
            "assertion": None,
            "description": "Is the configuration correct?",
            "positive": "Yes",
            "negative": "No",
        }],
    }))
    ctx.actions.write(results, json.encode({
        "results": [{"type": "assertion", "passed": ctx.attr.passed}],
    }))
    if ctx.attr.stale_lock:
        ctx.actions.write(lock, "")
    else:
        files_manifest = ctx.actions.declare_file(ctx.label.name + "_files.tsv")
        rules_manifest = ctx.actions.declare_file(ctx.label.name + "_rules.tsv")
        ctx.actions.write(files_manifest, "\n".join([
            "{}\t{}".format(file.short_path, file.path)
            for file in [analysis, results]
        ]) + "\n")
        ctx.actions.write(rules_manifest, "\n")
        ctx.actions.run(
            executable = ctx.executable._update_lock,
            inputs = [analysis, results, files_manifest, rules_manifest],
            outputs = [lock],
            env = {
                "MANUAL_ANALYSIS_FILES_MANIFEST": files_manifest.path,
                "MANUAL_ANALYSIS_RULES_MANIFEST": rules_manifest.path,
                "MANUAL_ANALYSIS_OUTPUT": lock.path,
            },
        )
    return [OutputGroupInfo(analysis = depset([analysis]), results = depset([results]), lock = depset([lock]))]

_manual_analysis_inputs = rule(
    implementation = _manual_analysis_inputs_impl,
    attrs = {
        "passed": attr.bool(default = True),
        "stale_lock": attr.bool(default = False),
        "_update_lock": attr.label(
            default = Label("@score_tooling//manual_analysis:update_lock"),
            executable = True,
            cfg = "exec",
        ),
    },
)

def _manual_analysis_integration_config_impl(ctx):
    review = ctx.attr.review
    report = ctx.attr.index[DependableElementLobsterInfo].lobster_report
    files_by_path = {file.path: file for file in review[DefaultInfo].files.to_list()}
    evidence = files_by_path[review[LobsterProvider].lobster_input.values()[0]]
    environment = dict(review[RunEnvironmentInfo].environment)
    environment.update({
        "MANUAL_ANALYSIS_REPORT": report.short_path,
        "MANUAL_ANALYSIS_EVIDENCE": evidence.short_path,
        "MANUAL_ANALYSIS_EXPECTED_ERROR": ctx.attr.expected_error,
    })
    configuration = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(configuration, json.encode(environment))
    runfiles = ctx.runfiles(files = [configuration, report, evidence])
    runfiles = runfiles.merge(review[DefaultInfo].default_runfiles)
    return [DefaultInfo(files = depset([configuration]), runfiles = runfiles)]

_manual_analysis_integration_config = rule(
    implementation = _manual_analysis_integration_config_impl,
    attrs = {
        "review": attr.label(providers = [LobsterProvider]),
        "index": attr.label(providers = [DependableElementLobsterInfo]),
        "expected_error": attr.string(),
    },
)

def manual_analysis_integration_test(name, passed = True, stale_lock = False, expected_error = ""):
    _manual_analysis_inputs(name = name + "_inputs", passed = passed, stale_lock = stale_lock, testonly = True)
    for group in ["analysis", "results", "lock"]:
        native.filegroup(name = name + "_" + group, srcs = [":" + name + "_inputs"], output_group = group, testonly = True)
    manual_analysis(
        name = name + "_review",
        contexts = [],
        analysis = ":" + name + "_analysis",
        results_file = ":" + name + "_results",
        lock_file = ":" + name + "_lock",
        tags = ["manual"],
    )
    dependable_element(
        name = name + "_element",
        architectural_design = [],
        components = [],
        dependability_analysis = [],
        integrity_level = "B",
        manual_analysis = [":" + name + "_review"],
        requirements = [],
        tests = [],
        deps = [":aou_derived_example_lib"],
        tags = ["manual"],
    )
    _manual_analysis_integration_config(
        name = name + "_config",
        testonly = True,
        review = ":" + name + "_review",
        index = ":" + name + "_element_index",
        expected_error = expected_error,
    )
    py_test(
        name = name,
        size = "small",
        srcs = [Label("//:test_dependable_element_manual_analysis.py")],
        main = Label("//:test_dependable_element_manual_analysis.py"),
        data = [":" + name + "_config"],
        env = {"MANUAL_ANALYSIS_CONFIG": "$(rootpath :{}_config)".format(name)},
        deps = ["@score_tooling//manual_analysis:manual_analysis_test_runner_lib"],
    )

def _manual_evidence_impl(ctx):
    output = ctx.actions.declare_file(ctx.label.name + ctx.attr.extension)
    ctx.actions.write(output, json.encode({
        "schema": "lobster-act-trace",
        "version": 3,
        "generator": "test",
        "data": [{
            "tag": "manualanalysis " + str(ctx.label),
            "location": {"kind": "file", "file": output.path, "line": None, "column": None},
            "name": "Manual verification: " + str(ctx.label),
            "refs": ["req " + requirement for requirement in ctx.attr.requirements],
            "just_up": [],
            "just_down": [],
            "just_global": [],
            "framework": "manual_analysis",
            "kind": "Manual Analysis Run",
            "status": "ok",
        }] if ctx.attr.requirements else [],
    }))
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
        "requirements": attr.string_list(),
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
    received_block = substitutions["{RECEIVED_AOU_BLOCK}"]
    if ctx.attr.expect_manual_analysis:
        asserts.true(env, 'activity "Manual Analysis"' in manual_block)
        asserts.true(env, "manual_evidence.lobster" in manual_block)
        asserts.false(env, 'trace to: "Component Requirements";' in manual_block)
        asserts.false(env, 'trace to: "Feature Requirements";' in manual_block)
        asserts.equals(env, bool(received_block), 'trace to: "Received AoUs";' in manual_block)
        asserts.false(env, 'trace to: "Assumptions of Use";' in manual_block)
        if received_block:
            asserts.true(
                env,
                'requires: "Manual Analysis" or "Forwarded AoUs";' in received_block,
            )
        asserts.false(env, 'trace to: "Received AoUs";' in component_block)
        asserts.false(env, "requires:" in component_block)
        asserts.false(env, "requires:" in feature_block)
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

def _received_aou_manual_analysis_test_impl(ctx):
    env = analysistest.begin(ctx)
    configs = [
        action
        for action in analysistest.target_actions(env)
        if any([output.basename == "de_traceability_config" for output in action.outputs.to_list()])
    ]
    asserts.equals(env, 1, len(configs))
    substitutions = configs[0].substitutions
    manual_block = substitutions["{MANUAL_ANALYSIS_BLOCK}"]
    received_block = substitutions["{RECEIVED_AOU_BLOCK}"]
    asserts.true(env, 'trace to: "Received AoUs";' in manual_block)
    asserts.false(env, 'trace to: "Component Requirements";' in manual_block)
    asserts.false(env, 'trace to: "Feature Requirements";' in manual_block)
    asserts.false(env, 'trace to: "Assumptions of Use";' in manual_block)
    asserts.equals(env, "", substitutions["{COMP_REQ_BLOCK}"])
    coverage_rule = "  requires: {};".format(" or ".join([
        '"{}"'.format(source)
        for source in ctx.attr.coverage_sources
    ]))
    asserts.true(env, coverage_rule in received_block)
    return analysistest.end(env)

received_aou_manual_analysis_test = analysistest.make(
    _received_aou_manual_analysis_test_impl,
    attrs = {
        "coverage_sources": attr.string_list(default = ["Manual Analysis", "Forwarded AoUs"]),
    },
)

def manual_analysis_test_suite(name):
    native.test_suite(
        name = name,
        tests = [
            ":manual_analysis_integration_assertion_failure_test",
            ":manual_analysis_integration_pass_test",
            ":manual_analysis_integration_stale_lock_test",
            ":manual_analysis_missing_output_test",
            ":manual_analysis_report_disabled_test",
            ":manual_analysis_report_release_test",
            ":manual_analysis_report_test",
            ":received_aou_manual_analysis_development_test",
            ":received_aou_manual_analysis_missing_test",
            ":received_aou_manual_analysis_pass_test",
            ":received_aou_manual_analysis_release_test",
        ],
    )
