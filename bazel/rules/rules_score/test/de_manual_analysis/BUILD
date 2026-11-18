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

load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "component", "dependable_element")
load(
    "//:dependable_element_manual_analysis_test.bzl",
    "manual_analysis_integration_test",
    "manual_analysis_missing_output_test",
    "manual_analysis_report_test",
    "manual_analysis_test_suite",
    "manual_evidence",
    "received_aou_component_requirements_test",
    "received_aou_manual_analysis_test",
)
load("//:lobster_ci_report_test.bzl", "lobster_ci_report_test")

package(default_visibility = ["//visibility:public"])

manual_evidence(
    name = "manual_evidence",
    testonly = True,
)

manual_evidence(
    name = "manual_evidence_json",
    testonly = True,
    extension = ".json",
)

manual_evidence(
    name = "manual_evidence_runfiles",
    testonly = True,
    extension = ".json",
    runfiles_only = True,
)

dependable_element(
    name = "test_dependable_element_manual_analysis",
    architectural_design = ["//:arch_design_elem"],
    components = ["//:test_component"],
    dependability_analysis = ["//:dependability_analysis_target"],
    integrity_level = "B",
    manual_analysis = [":manual_evidence"],
    maturity = "development",
    requirements = ["//:feat_req"],
    tags = ["manual"],
    tests = [],
)

dependable_element(
    name = "test_dependable_element_manual_analysis_release",
    architectural_design = ["//:arch_design_elem"],
    components = ["//:test_component"],
    dependability_analysis = ["//:dependability_analysis_target"],
    integrity_level = "B",
    manual_analysis = [
        ":manual_evidence",
        ":manual_evidence_json",
    ],
    requirements = ["//:feat_req"],
    tags = ["manual"],
    tests = [],
    deps = ["//:aou_derived_example_lib"],
)

manual_analysis_report_test(
    name = "manual_analysis_report_test",
    target_under_test = ":test_dependable_element_manual_analysis_index",
)

manual_analysis_report_test(
    name = "manual_analysis_report_release_test",
    extra_evidence = ["manual_evidence_json.json"],
    target_under_test = ":test_dependable_element_manual_analysis_release_index",
)

manual_analysis_report_test(
    name = "manual_analysis_report_disabled_test",
    expect_manual_analysis = False,
    target_under_test = "//:test_dependable_element_index",
)

dependable_element(
    name = "test_dependable_element_manual_analysis_runfiles",
    architectural_design = ["//:arch_design_elem"],
    components = ["//:test_component"],
    dependability_analysis = ["//:dependability_analysis_target"],
    integrity_level = "B",
    manual_analysis = [":manual_evidence_runfiles"],
    maturity = "development",
    requirements = ["//:feat_req"],
    tags = ["manual"],
    tests = [],
)

manual_analysis_missing_output_test(
    name = "manual_analysis_missing_output_test",
    target_under_test = ":test_dependable_element_manual_analysis_runfiles_index",
)

manual_evidence(
    name = "received_aou_manual_evidence",
    testonly = True,
    requirements = ["AouRootCauseAous.ConfigurationIsCorrect"],
)

manual_evidence(
    name = "received_aou_manual_evidence_missing",
    testonly = True,
)

dependable_element(
    name = "received_aou_manual_analysis_release",
    architectural_design = [],
    components = [],
    dependability_analysis = [],
    integrity_level = "B",
    manual_analysis = [":received_aou_manual_evidence"],
    requirements = [],
    tags = ["manual"],
    tests = [],
    deps = ["//:aou_derived_example_lib"],
)

dependable_element(
    name = "received_aou_manual_analysis_development",
    architectural_design = [],
    components = [],
    dependability_analysis = [],
    integrity_level = "B",
    manual_analysis = [":received_aou_manual_evidence"],
    maturity = "development",
    requirements = [],
    tags = ["manual"],
    tests = [],
    deps = ["//:aou_derived_example_lib"],
)

dependable_element(
    name = "received_aou_manual_analysis_missing",
    architectural_design = [],
    components = [],
    dependability_analysis = [],
    integrity_level = "B",
    manual_analysis = [":received_aou_manual_evidence_missing"],
    requirements = [],
    tags = ["manual"],
    tests = [],
    deps = ["//:aou_derived_example_lib"],
)

received_aou_manual_analysis_test(
    name = "received_aou_manual_analysis_release_test",
    target_under_test = ":received_aou_manual_analysis_release_index",
)

received_aou_manual_analysis_test(
    name = "received_aou_manual_analysis_development_test",
    coverage_sources = ["Manual Analysis"],
    target_under_test = ":received_aou_manual_analysis_development_index",
)

lobster_ci_report_test(
    name = "received_aou_manual_analysis_pass_test",
    expect_ci_success = True,
    index = ":received_aou_manual_analysis_release_index",
)

lobster_ci_report_test(
    name = "received_aou_manual_analysis_missing_test",
    contains = [
        "fixtures/aou_root_cause/aou.trlc",
        "missing reference to Forwarded AoUs or Manual Analysis",
    ],
    expect_ci_failure = True,
    index = ":received_aou_manual_analysis_missing_index",
)

# A component requirement covers the received AoU through `derived_from`, with
# and without manual analysis evidence for the same AoU.
component(
    name = "received_aou_component",
    testonly = True,
    components = [],
    requirements = ["//:comp_req_measure_reqs"],
    tags = ["manual"],
    tests = [],
)

dependable_element(
    name = "received_aou_component_requirements_only",
    architectural_design = [],
    components = [":received_aou_component"],
    dependability_analysis = [],
    integrity_level = "B",
    maturity = "development",
    requirements = [],
    tags = ["manual"],
    tests = [],
    deps = ["//:aou_derived_example_lib"],
)

dependable_element(
    name = "received_aou_component_requirements_both",
    architectural_design = [],
    components = [":received_aou_component"],
    dependability_analysis = [],
    integrity_level = "B",
    manual_analysis = [":received_aou_manual_evidence"],
    maturity = "development",
    requirements = [],
    tags = ["manual"],
    tests = [],
    deps = ["//:aou_derived_example_lib"],
)

received_aou_component_requirements_test(
    name = "received_aou_component_requirements_only_test",
    target_under_test = ":received_aou_component_requirements_only_index",
)

received_aou_component_requirements_test(
    name = "received_aou_component_requirements_both_test",
    expect_manual_analysis = True,
    target_under_test = ":received_aou_component_requirements_both_index",
)

lobster_ci_report_test(
    name = "received_aou_component_requirements_only_pass_test",
    contains = ["fixtures/comp_req_measure/comp_req.trlc"],
    index = ":received_aou_component_requirements_only_index",
    lacks = ["fixtures/aou_root_cause/aou.trlc"],
)

lobster_ci_report_test(
    name = "received_aou_component_requirements_both_pass_test",
    contains = ["fixtures/comp_req_measure/comp_req.trlc"],
    index = ":received_aou_component_requirements_both_index",
    lacks = ["fixtures/aou_root_cause/aou.trlc"],
)

manual_analysis_integration_test(name = "manual_analysis_integration_pass_test")

manual_analysis_integration_test(
    name = "manual_analysis_integration_assertion_failure_test",
    expected_error = "Final manual analysis assertion is not positive.",
    passed = False,
)

manual_analysis_integration_test(
    name = "manual_analysis_integration_stale_lock_test",
    expected_error = "Manual analysis lock file is out of date.",
    stale_lock = True,
)

manual_analysis_test_suite(name = "manual_analysis_tests")
