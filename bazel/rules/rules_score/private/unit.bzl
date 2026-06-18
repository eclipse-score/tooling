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
Unit build rules for S-CORE projects.

This module provides macros and rules for defining software units
following S-CORE process guidelines. A unit is the smallest testable
software element with associated design, implementation, and tests.
"""

load("@bazel_skylib//rules:common_settings.bzl", "BuildSettingInfo")
load("@lobster//:lobster.bzl", "subrule_gtest_report")
load("@rules_cc//cc:find_cc_toolchain.bzl", "use_cc_toolchain")
load("@rules_cc//cc/common:cc_info.bzl", "CcInfo")
load("@rules_rust//rust:defs.bzl", "rust_common")
load("//bazel/rules/rules_score:providers.bzl", "CcDependencyInfo", "CertifiedScope", "SphinxSourcesInfo", "UnitDesignInfo", "UnitInfo")
load("//cpp/libclang:cpp_parser.bzl", "cpp_parser_action_internal_attrs", "cpp_parser_target_aspects", "has_cpp_parser_inputs", "run_cpp_parser_action")
load(":cc_dependency_aspect.bzl", "cc_dependencies_aspect")

def _run_implementation_cpp_parser(ctx, impl, output_prefix):
    return run_cpp_parser_action(
        ctx,
        target = impl,
        output_prefix = output_prefix,
        tool = ctx.attr._tool,
        libclang = ctx.file._libclang,
        llvm_cxx_builtin_include = ctx.attr._llvm_cxx_builtin_include,
        llvm_extra_config_site = ctx.attr._llvm_extra_config_site,
        log_level = ctx.attr._log_level[BuildSettingInfo].value,
    )

def _target_output_prefix(ctx, target):
    package_name = target.label.package.replace("/", "_")
    target_name = target.label.name.replace("/", "_")
    return "{}_{}_{}".format(ctx.label.name, package_name, target_name)

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _unit_impl(ctx):
    """Implementation for unit rule.

    Collects unit design artifacts, implementation targets, and tests
    and provides them through the UnitInfo provider.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and UnitInfo
    """

    # Collect design files from unit_design targets
    design_files = []
    sphinx_design_deps = []
    design_static_fbs = []
    design_dynamic_fbs = []
    for design_target in ctx.attr.unit_design:
        if SphinxSourcesInfo in design_target:
            design_files.append(design_target[SphinxSourcesInfo].srcs)
            sphinx_design_deps.append(design_target[SphinxSourcesInfo].deps)
        if UnitDesignInfo in design_target:
            design_static_fbs.append(design_target[UnitDesignInfo].static)
            design_dynamic_fbs.append(design_target[UnitDesignInfo].dynamic)

    design_depset = depset(transitive = design_files)
    design_static_fbs_depset = depset(transitive = design_static_fbs)
    design_dynamic_fbs_depset = depset(transitive = design_dynamic_fbs)

    # Generate FBS files for implementation targets supported by the libclang parser.
    implementation_class_fbs = []
    collected_dependent_labels = []
    for impl in ctx.attr.implementation:
        if CcDependencyInfo in impl:
            collected_dependent_labels.append(impl[CcDependencyInfo].dependencies)

        if not has_cpp_parser_inputs(impl):
            continue

        output_prefix = _target_output_prefix(ctx, impl)
        parser_outputs = _run_implementation_cpp_parser(ctx, impl, output_prefix)
        implementation_class_fbs.append(parser_outputs.class_fbs)

    # Run each test executable via subrule_gtest_report and collect the XML outputs
    xml_files = []
    for test_target in ctx.attr.tests:
        unique_name = "{}_gtest_report".format(
            _target_output_prefix(ctx, test_target),
        )
        xml = subrule_gtest_report(
            unique_name,
            test_target.files_to_run.executable,
            test_target.default_runfiles.files,
        )
        xml_files.append(xml)

    tests_depset = depset(xml_files)

    # Combine all files for DefaultInfo
    all_files = depset(
        xml_files + implementation_class_fbs,
        transitive = [design_depset],
    )

    return [
        DefaultInfo(files = all_files),
        CertifiedScope(transitive_scopes = depset(ctx.attr.scope)),
        UnitInfo(
            name = ctx.label.name,
            unit_design = design_depset,
            unit_design_static_fbs = design_static_fbs_depset,
            unit_design_dynamic_fbs = design_dynamic_fbs_depset,
            implementation = depset(ctx.attr.implementation),
            tests = tests_depset,
            dependent_labels = depset(transitive = collected_dependent_labels),
        ),
        SphinxSourcesInfo(
            srcs = all_files,
            deps = depset(transitive = [all_files] + sphinx_design_deps),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_unit_attrs = {
    "unit_design": attr.label_list(
        mandatory = True,
        providers = [UnitDesignInfo],
        doc = "Unit design artifacts (unit_design targets only)",
    ),
    "implementation": attr.label_list(
        mandatory = True,
        providers = [[CcInfo], [rust_common.crate_info]],
        aspects = [cc_dependencies_aspect] + cpp_parser_target_aspects(),
        doc = "Implementation targets (cc_library, cc_binary, rust_library, rust_binary, etc.).",
    ),
    "scope": attr.string_list(
        default = [],
        doc = "Additional not explicitly named targets which are needed for the unit implementation",
    ),
    "tests": attr.label_list(
        mandatory = True,
        cfg = "exec",
        doc = "Test targets that verify the unit (cc_test, py_test, rust_test, etc.)",
    ),
}

_unit_attrs.update(cpp_parser_action_internal_attrs())

_unit = rule(
    implementation = _unit_impl,
    doc = "Defines a software unit with design, implementation, and tests for S-CORE process compliance",
    subrules = [subrule_gtest_report],
    attrs = _unit_attrs,
    toolchains = use_cc_toolchain(),
    fragments = ["cpp"],
)

# ============================================================================
# Public Macro
# ============================================================================

def unit(
        name,
        unit_design,
        implementation,
        tests,
        scope = [],
        testonly = True,
        **kwargs):
    """Define a software unit following S-CORE process guidelines.

    A unit is the smallest testable software element in the S-CORE process.
    It consists of:
    - Unit design: Design documentation and diagrams
    - Implementation: Source code that realizes the design
    - Tests: Test cases that verify the implementation

    Args:
        name: The name of the unit. Used as the target name.
        unit_design: List of labels to unit_design targets that describe the
            unit's internal structure and behavior.
        implementation: List of labels to Bazel targets representing the actual
            implementation (cc_library, cc_binary, rust_library, rust_binary, etc.).
        scope: Optional list of additional targets needed for the unit implementation
            but not explicitly named in the implementation list. Default is empty list.
        tests: List of labels to Bazel test targets (cc_test, rust_test, etc.)
            that verify the unit implementation.
        testonly: If true, only testonly targets can depend on this unit. Set to true
            when the unit depends on testonly targets like tests.
        visibility: Bazel visibility specification for the unit target.

    Example:
        ```python
        unit(
            name = "kvs_unit1",
            unit_design = [":kvs_unit_design"],
            implementation = [
                "//persistency/kvs:lib1",
                "//persistency/kvs:lib2",
                "//persistency/kvs:lib3",
            ],
            tests = ["//persistency/kvs/tests:score_kvs_component_tests"],
            visibility = ["//visibility:public"],
        )
        ```
    """
    _unit(
        name = name,
        unit_design = unit_design,
        implementation = implementation,
        scope = scope,
        tests = tests,
        testonly = testonly,
        **kwargs
    )
