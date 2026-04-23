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

"""
Component build rules for S-CORE projects.

This module provides macros and rules for defining software components
following S-CORE process guidelines. A component consists of multiple units
with associated requirements and tests.
"""

load("@lobster//:lobster.bzl", "subrule_lobster_gtest", "subrule_lobster_html_report", "subrule_lobster_report")
load("//bazel/rules/rules_score:providers.bzl", "CertifiedScope", "ComponentInfo", "ComponentRequirementsInfo", "FeatureRequirementsInfo", "SphinxSourcesInfo", "UnitInfo")
load("//bazel/rules/rules_score/private:lobster_config.bzl", "format_lobster_sources")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _component_impl(ctx):
    """Implementation for component rule.

    Collects component requirements, units, and tests and provides them
    through the ComponentInfo provider.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and ComponentInfo
    """

    # -------------------------------------------------------------------------
    # Sphinx Docs: collect RST sources from component_requirements targets
    # and bubble up SphinxSourcesInfo from sub-components/units
    # -------------------------------------------------------------------------
    req_sphinx_files = []
    for req_target in ctx.attr.requirements:
        if SphinxSourcesInfo in req_target:
            req_sphinx_files.append(req_target[SphinxSourcesInfo].srcs)

    component_sphinx_files = []
    for component in ctx.attr.components:
        if SphinxSourcesInfo in component:
            component_sphinx_files.append(component[SphinxSourcesInfo].deps)

    req_sphinx_depset = depset(transitive = req_sphinx_files)
    sphinx_depset = depset(transitive = req_sphinx_files + component_sphinx_files)

    # -------------------------------------------------------------------------
    # Lobster Tracing: collect .lobster files from component_requirements targets
    # and feature_requirements targets (needed to resolve derived_from references)
    # -------------------------------------------------------------------------
    req_lobster_files = []
    feat_req_lobster_files = []
    for req_target in ctx.attr.requirements:
        if ComponentRequirementsInfo in req_target:
            req_lobster_files.append(req_target[ComponentRequirementsInfo].srcs)
        if FeatureRequirementsInfo in req_target:
            feat_req_lobster_files.append(req_target[FeatureRequirementsInfo].srcs)

    req_lobster_depset = depset(transitive = req_lobster_files)
    feat_req_lobster_depset = depset(transitive = feat_req_lobster_files)

    # Collect nested components
    components_depset = depset(ctx.attr.components)

    # -------------------------------------------------------------------------
    # Collect Dependencies and Certification Scopes from sub-components/units
    # -------------------------------------------------------------------------
    collected_certified_scopes = []
    collected_dependencies = []
    for component in ctx.attr.components:
        if UnitInfo in component:
            if component[UnitInfo].dependent_labels:
                collected_dependencies.append(component[UnitInfo].dependent_labels)
        if ComponentInfo in component:
            if component[ComponentInfo].dependent_labels:
                collected_dependencies.append(component[ComponentInfo].dependent_labels)
        if CertifiedScope in component:
            collected_certified_scopes.append(component[CertifiedScope].transitive_scopes)

    # -------------------------------------------------------------------------
    # Propagate Tracing Files: collect .lobster files from sub-components
    # and unit test results from units (to be converted to gtest.lobster below)
    # -------------------------------------------------------------------------
    collected_unit_test_files = []
    collected_tests = []
    collected_architecture = []
    for component in ctx.attr.components:
        if UnitInfo in component:
            collected_unit_test_files.append(component[UnitInfo].tests)
        if ComponentInfo in component:
            if component[ComponentInfo].tests:
                collected_tests.append(component[ComponentInfo].tests)
            if component[ComponentInfo].architecture:
                collected_architecture.append(component[ComponentInfo].architecture)

    # Convert unit test XML files to a single gtest.lobster file
    gtest_lobster_file, provider = subrule_lobster_gtest(depset(transitive = collected_unit_test_files).to_list())

    # -------------------------------------------------------------------------
    # Architecture Lobster: generate architecture.lobster for this component
    # Creates a single lobster item representing the component, referencing
    # all component requirements allocated to it through Bazel.
    # -------------------------------------------------------------------------
    arch_lobster_file = None
    if req_lobster_files:
        arch_lobster_file = ctx.actions.declare_file(ctx.attr.name + "_architecture.lobster")
        arch_to_reqs_args = ctx.actions.args()
        arch_to_reqs_args.add("--component-name", "//%s:%s" % (ctx.label.package, ctx.label.name))
        arch_to_reqs_args.add("--build-file", ctx.label.package + "/BUILD")
        arch_to_reqs_args.add("--output", arch_lobster_file)
        arch_to_reqs_args.add("--req-lobster")
        arch_to_reqs_args.add_all(req_lobster_depset)

        ctx.actions.run(
            inputs = req_lobster_depset.to_list(),
            outputs = [arch_lobster_file],
            executable = ctx.executable._arch_to_reqs_from_lobster,
            arguments = [arch_to_reqs_args],
            progress_message = "Generating architecture lobster for %s" % ctx.label,
        )

    # -------------------------------------------------------------------------
    # Generate Lobster Configuration: expand static template, substituting
    # only the source file lists (the structure is fixed per variant).
    # -------------------------------------------------------------------------
    comp_req_lobster_files = req_lobster_depset.to_list()
    feat_req_lobster_files_list = feat_req_lobster_depset.to_list()
    all_lobster_inputs = list(comp_req_lobster_files) + feat_req_lobster_files_list

    if arch_lobster_file:
        all_lobster_inputs.append(arch_lobster_file)

    all_lobster_inputs.append(gtest_lobster_file)

    lobster_config = ctx.actions.declare_file(ctx.attr.name + "_traceability_config")
    ctx.actions.expand_template(
        template = ctx.file._lobster_comp_template,
        output = lobster_config,
        substitutions = {
            "{FEAT_REQ_SOURCES}": format_lobster_sources(feat_req_lobster_files_list),
            "{COMP_REQ_SOURCES}": format_lobster_sources(comp_req_lobster_files),
            "{ARCH_SOURCES}": format_lobster_sources([arch_lobster_file] if arch_lobster_file else []),
            "{UNIT_TEST_SOURCES}": format_lobster_sources([gtest_lobster_file]),
        },
    )

    # -------------------------------------------------------------------------
    # Lobster Report Build: produce .lobster report, HTML
    # -------------------------------------------------------------------------
    lobster_report = subrule_lobster_report(all_lobster_inputs, lobster_config)
    lobster_html_report = subrule_lobster_html_report(lobster_report)

    test_executable = ctx.actions.declare_file("{}_lobster_ci_test_executable".format(ctx.attr.name))
    ctx.actions.write(
        output = test_executable,
        content = "set -o pipefail; {} {}".format(
            ctx.executable._lobster_ci_report.short_path,
            lobster_report.short_path,
        ),
    )

    return [
        # DefaultInfo: lobster report as build output + CI test executable
        DefaultInfo(
            runfiles = ctx.runfiles(
                files = [
                    ctx.executable._lobster_ci_report,
                    lobster_report,
                ],
            ).merge(ctx.attr._lobster_ci_report[DefaultInfo].default_runfiles),
            files = depset([lobster_report, lobster_html_report]),
            executable = test_executable,
        ),
        # CertifiedScope: propagate certification scopes upward
        CertifiedScope(transitive_scopes = depset(transitive = collected_certified_scopes)),
        # ComponentInfo: lobster traceability files for requirements, architecture, and tests; propagated up to dependable_element
        ComponentInfo(
            name = ctx.label.name,
            requirements = req_lobster_depset,
            components = components_depset,
            tests = depset(
                [gtest_lobster_file],
                transitive = collected_tests,
            ),
            architecture = depset(
                [arch_lobster_file] if arch_lobster_file else [],
                transitive = collected_architecture,
            ),
            dependent_labels = depset(transitive = collected_dependencies),
        ),
        # SphinxSourcesInfo: RST sources from component requirements + transitive sources from sub-components/units
        SphinxSourcesInfo(
            srcs = req_sphinx_depset,
            deps = sphinx_depset,
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_component_test = rule(
    implementation = _component_impl,
    doc = "Defines a software component composed of multiple units for S-CORE process compliance",
    attrs = {
        "requirements": attr.label_list(
            mandatory = True,
            providers = [[ComponentRequirementsInfo], [FeatureRequirementsInfo]],
            doc = "Component requirements artifacts (component_requirements or feature_requirements targets)",
        ),
        "components": attr.label_list(
            providers = [[ComponentInfo], [UnitInfo]],
            doc = "Nested component or unit targets (components can contain both components and units)",
        ),
        "tests": attr.label_list(
            mandatory = True,
            doc = "Component-level integration test targets",
        ),
        "_lobster_ci_report": attr.label(
            default = "@lobster//:lobster-ci-report",
            executable = True,
            cfg = "exec",
        ),
        "_arch_to_reqs_from_lobster": attr.label(
            default = Label("//bazel/rules/rules_score:arch_to_reqs_from_lobster"),
            executable = True,
            cfg = "exec",
            doc = "Tool to extract component requirements and generate architecture .lobster items for component traceability",
        ),
        "_lobster_comp_template": attr.label(
            default = Label("//bazel/rules/rules_score/lobster/config:lobster_component_template"),
            allow_single_file = True,
            doc = "Lobster config template for component traceability.",
        ),
    },
    subrules = [subrule_lobster_gtest, subrule_lobster_report, subrule_lobster_html_report],
    test = True,
)

# ============================================================================
# Public Macro
# ============================================================================

def component(
        name,
        tests = [],
        requirements = None,
        components = [],
        testonly = True,
        visibility = None,
        tags = []):
    """Define a software component following S-CORE process guidelines.

    A component is a collection of related units that together provide
    a specific functionality. It consists of:
    - Component requirements: Requirements specification for the component
    - Components: Nested components (for hierarchical structures)
    - Tests: Integration tests that verify the component as a whole

    Args:
        name: The name of the component. Used as the target name.
        requirements: List of labels to component_requirements targets
            that define the requirements for this component.
        components: List of labels to nested component targets (for hierarchical
            component structures).
        tests: List of labels to Bazel test targets that verify the component
            integration.
        testonly: If true, only testonly targets can depend on this component.
        visibility: Bazel visibility specification for the component target.

    Example:
        ```python
        component(
            name = "kvs_component",
            requirements = [":kvs_component_requirements"],
            components = [":kvs_unit1", ":kvs_unit2"],
            tests = ["//persistency/kvs/tests:score_kvs_component_integration_tests"],
            visibility = ["//visibility:public"],
        )
        ```
    """

    _component_test(
        name = name,
        requirements = requirements,
        components = components,
        tests = tests,
        testonly = testonly,
        visibility = visibility,
        tags = tags,
    )
