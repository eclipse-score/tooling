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

load("@pip_tooling_test//:requirements.bzl", "requirement")
load("@rules_cc//cc:defs.bzl", "cc_binary", "cc_library", "cc_test")
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "architectural_design",
    "assumed_system_requirements",
    "assumptions_of_use",
    "component",
    "component_requirements",
    "dependability_analysis",
    "dependable_element",
    "feature_requirements",
    "fmea",
    "sphinx_module",
    "unit",
    "unit_design",
)
load("@score_tooling//bazel/rules/rules_score:sphinx_toolchain.bzl", "score_sphinx_toolchain")
load("@score_tooling//cpp/libclang:libclang_toolchain.bzl", "libclang_toolchain")
load("@trlc//:trlc.bzl", "trlc_requirements", "trlc_requirements_test")
load(
    ":html_generation_test.bzl",
    "auto_config_generation_test",
    "basic_html_generation_test",
    "explicit_config_test",
    "html_merging_test",
    "module_dependencies_test",
    "needs_generation_test",
    "needs_transitive_test",
    "providers_test",
    "sphinx_module_test_suite",
)
load(
    ":lobster_config_test.bzl",
    "lobster_config_test_suite",
)
load(
    ":requirements_image_test.bzl",
    "image_srcs_sphinx_sources_test",
    "requirements_image_test_suite",
)
load(
    ":requirements_rst_test.bzl",
    "aous_rst_provider_test",
    "aous_rst_sphinx_test",
    "asr_rst_output_test",
    "asr_rst_provider_test",
    "asr_rst_sphinx_test",
    "comp_req_rst_provider_test",
    "comp_req_rst_sphinx_test",
    "feat_req_rst_provider_test",
    "feat_req_rst_sphinx_test",
    "requirements_rst_test_suite",
)
load(
    ":score_module_providers_test.bzl",
    "config_auto_generation_test",
    "config_explicit_usage_test",
    "deps_html_merging_test",
    "deps_needs_collection_test",
    "score_needs_info_fields_test",
    "score_needs_transitive_collection_test",
    "score_needs_with_deps_test",
    "score_toolchain_override_test",
    "sphinx_module_info_fields_test",
    "sphinx_module_providers_test_suite",
    "transitive_modules_dedup_test",
    "two_phase_html_second_test",
    "two_phase_needs_first_test",
    "worker_gating_html_test",
    "worker_gating_needs_test",
)
load(
    ":seooc_test.bzl",
    "seooc_all_sources_relocated_test",
    "seooc_artifacts_copied_test",
    "seooc_index_file_provider_test",
    "seooc_index_generation_test",
    "seooc_multi_index_files_exist_test",
    "seooc_needs_provider_test",
    "seooc_sphinx_entry_point_is_root_index_test",
    "seooc_sphinx_module_generated_test",
)
load(
    ":unit_component_test.bzl",
    "component_excludes_feature_req_docs_test",
    "component_provider_test",
    "component_sphinx_sources_test",
    "component_test_case_coverage_lock_test",
    "dependable_element_test_case_coverage_lock_check_action_test",
    "unit_component_test_suite",
    "unit_provider_test",
    "unit_sphinx_sources_test",
)

package(default_visibility = ["//visibility:public"])

# ============================================================================
# Sphinx Toolchain — uses score_docs_as_code so score_metamodel is importable
# ============================================================================

exports_files(["template/conf.template.py"])

score_sphinx_toolchain(
    name = "score_toolchain",
    conf_template = "//:template/conf.template.py",
    deps = [
        "@score_docs_as_code//src:plantuml_for_python",
        "@score_docs_as_code//src/extensions/score_sphinx_bundle",
        "@score_tooling//bazel/rules/rules_score:bazel_sphinx_needs",
        "@score_tooling//bazel/rules/rules_score:sphinx_module_ext",
        "@score_tooling//plantuml/sphinx/clickable_plantuml",
        requirement("sphinx-rtd-theme"),
    ],
)

# ============================================================================
# libclang Toolchain — this integrating repo supplies its own libclang/C++
# toolchain for the score libclang-based C/C++ parser.
# ============================================================================
libclang_toolchain(
    name = "test_libclang",
    cc_toolchain = "@llvm_toolchain//:cc-clang-x86_64-linux",
    libclang = "@llvm_toolchain_llvm//:lib/libclang.so",
)

toolchain(
    name = "test_libclang_toolchain",
    toolchain = ":test_libclang",
    toolchain_type = "@score_tooling//cpp/libclang:libclang_toolchain_type",
)

# ============================================================================
# Test Fixtures - Module Definitions
# ============================================================================

# Test 1: Multi-Module Aggregation
# Dependency graph: module_a_lib -> module_b_lib -> module_c_lib
#                   module_a_lib -> module_c_lib (also direct)
# module_d_lib is a diamond dependency reachable only transitively, via
# module_b_lib AND module_c_lib (never directly from module_a_lib) -- a
# regression fixture for the HTML-merge flattening: module_d_lib's HTML must
# be merged into module_a_lib's published site exactly once, not once per
# path that reaches it.
sphinx_module(
    name = "module_d_lib",
    srcs = glob(["fixtures/module_d/*.rst"]),
    index = "fixtures/module_d/index.rst",
)

sphinx_module(
    name = "module_c_lib",
    srcs = glob(["fixtures/module_c/*.rst"]),
    index = "fixtures/module_c/index.rst",
    deps = [":module_d_lib"],
)

sphinx_module(
    name = "module_b_lib",
    srcs = glob(["fixtures/module_b/*.rst"]),
    index = "fixtures/module_b/index.rst",
    deps = [
        ":module_c_lib",
        ":module_d_lib",
    ],
)

sphinx_module(
    name = "module_a_lib",
    srcs = glob(["fixtures/module_a/*.rst"]),
    index = "fixtures/module_a/index.rst",
    deps = [
        ":module_b_lib",
        ":module_c_lib",
    ],
)

# Regression fixture for two-hop needs resolution: module_e_lib depends only
# on module_b_lib, with no direct edge to module_c_lib or module_d_lib, yet
# its index.rst references a need defined in module_d_lib (reachable only via
# module_b_lib -> module_d_lib). needs_external_needs.json must be built from
# the full transitive closure of needs modules (SphinxNeedsInfo.needs_modules)
# for this to resolve -- building it from direct deps only (the pre-fix
# behavior) can never see past one hop.
sphinx_module(
    name = "module_e_lib",
    srcs = glob(["fixtures/module_e/*.rst"]),
    index = "fixtures/module_e/index.rst",
    deps = [":module_b_lib"],
)

# Fixture with allow_persistent_workers = True, used only to assert on the
# resulting Sphinx-build action argv: the HTML pass must drop "--jobs auto"
# (see worker_enabled in sphinx_module.bzl's _add_sphinx_args), while the
# needs pass -- which never opts into worker mode regardless of this attr,
# since its source dir is the real checkout, not a generated tree -- must
# keep it. No target ever sets this True outside this test fixture; workers
# stay off by default across the whole repo.
sphinx_module(
    name = "module_worker_enabled_lib",
    testonly = True,
    srcs = glob(["fixtures/module_c/*.rst"]),
    allow_persistent_workers = True,
    index = "fixtures/module_c/index.rst",
)

# Test 3: SEooC (Safety Element out of Context) Module
# Tests the score_component macro with S-CORE process artifacts

# - Feature Requirements:     wp__requirements_feat
assumed_system_requirements(
    name = "asr_trlc",
    srcs = ["fixtures/seooc_test/assumed_system_requirements.trlc"],
)

feature_requirements(
    name = "feat_req",
    srcs = ["fixtures/seooc_test/feature_requirements.trlc"],
    deps = [":asr_trlc"],
)

# - Component Requirements:   wp__requirements_comp
component_requirements(
    name = "comp_req",
    srcs = ["fixtures/seooc_test/component_requirements.trlc"],
    deps = [
        ":asr_trlc",
        ":feat_req",
    ],
)

# - Assumptions of Use:       wp__requirements_comp_aou
assumptions_of_use(
    name = "aous",
    srcs = ["fixtures/seooc_test/assumptions_of_use.rst"],
)

# - Architecture Design:      wp__component_arch
architectural_design(
    name = "arch_design",
    dynamic = ["fixtures/seooc_test/dynamic_architecture.rst"],
    static = [
        "fixtures/seooc_test_lib.puml",
        "fixtures/seooc_test/static_architecture.rst",
    ],
)

architectural_design(
    name = "arch_design_elem",
    static = ["fixtures/test_dependable_element.puml"],
)

architectural_design(
    name = "arch_design_nested",
    static = ["fixtures/test_dependable_element_nested.puml"],
)

# - Safety Analysis (DFA):    wp__sw_component_dfa
# - Safety Analysis (FMEA):   wp__sw_component_fmea
dependability_analysis(
    name = "dependability_analysis_target",
    arch_design = ":arch_design",
    dfa = ["fixtures/seooc_test/dfa.rst"],
    fmea = [":samplelibrary_safety_analysis"],
)

fmea(
    name = "samplelibrary_safety_analysis",
    # TODO: add failure modes, control measures, and root causes here
    # failuremodes = [],
    # controlmeasures = [], # can be AoUs or requirements
    # root_causes = [],
    arch_design = ":arch_design",
)

architectural_design(
    name = "arch_design_dep",
    static = ["fixtures/dep_seooc_lib.puml"],
)

# Live example backing the "End-to-End Clickable Diagram Example" documented
# in //plantuml/sphinx/clickable_plantuml/README.md: overview.puml references
# `Proxy`, which proxy_detail.puml defines, so clickable_plantuml should
# inject a real link between the two rendered diagrams. Regression-checked by
# :clickable_example_link_rendered_test below.
architectural_design(
    name = "arch_design_clickable_example_overview",
    static = ["fixtures/clickable_example/overview.puml"],
)

architectural_design(
    name = "arch_design_clickable_example_proxy_detail",
    static = ["fixtures/clickable_example/proxy_detail.puml"],
)

dependable_element(
    name = "clickable_example_lib",
    architectural_design = [
        ":arch_design_clickable_example_overview",
        ":arch_design_clickable_example_proxy_detail",
    ],
    assumptions_of_use = [":aous"],
    components = [],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    # This fixture intentionally reuses the same top-level id ("Proxy") as
    # both a reference (overview.puml) and a definition (proxy_detail.puml)
    # to exercise clickable_plantuml's cross-diagram id resolution - exactly
    # the scenario documented in the clickable_plantuml README. The
    # dependable-element process-compliance validator doesn't understand
    # define-vs-reference semantics and flags this as a duplicate id / missing
    # SEooC package, so validation findings are downgraded to warnings here;
    # this fixture is purely illustrative and isn't a real SEooC.
    maturity = "development",
    requirements = [":feat_req"],
    tests = [],
    deps = [],
)

# Live example of clickable_plantuml's cross-diagram-type "interface" linking:
# interface_overview.puml (a component diagram) references `InternalInterface`
# via a `-(` binding on a unit, and interface_detail.puml (auto-detected as a
# class diagram because the interface has a method) defines it - both share
# the FQN `package_a.InternalInterface` because ids are rooted at the
# enclosing `package_a` package, not at the `@startuml` name. Mirrors
# validation/core/integration_test/component_internal_api/positive_interface_match.
# Regression-checked by :interface_example_link_rendered_test below.
architectural_design(
    name = "arch_design_interface_example",
    internal_api = ["fixtures/clickable_example/interface_detail.puml"],
    static = ["fixtures/clickable_example/interface_overview.puml"],
)

dependable_element(
    name = "interface_example_lib",
    architectural_design = [":arch_design_interface_example"],
    assumptions_of_use = [":aous"],
    components = [],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    # Downgraded to warnings for the same reason as clickable_example_lib
    # above: this fixture intentionally reuses "InternalInterface" as both a
    # reference and a definition, which the dependable-element validator
    # can't distinguish from an accidental duplicate id.
    maturity = "development",
    requirements = [":feat_req"],
    tests = [],
    deps = [],
)

# Live example of clickable_plantuml's pure class-diagram linking:
# class_overview.puml references `AuditTrail` as a bare, member-less class,
# and class_detail.puml defines it (it has a method, making it the
# elaboration site). Regression-checked by :class_example_link_rendered_test
# below.
architectural_design(
    name = "arch_design_class_example",
    static = [
        "fixtures/clickable_example/class_overview.puml",
        "fixtures/clickable_example/class_detail.puml",
    ],
)

dependable_element(
    name = "class_example_lib",
    architectural_design = [":arch_design_class_example"],
    assumptions_of_use = [":aous"],
    components = [],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    # Downgraded to warnings for the same reason as clickable_example_lib
    # above: this fixture intentionally reuses "AuditTrail" as both a
    # reference and a definition, which the dependable-element validator
    # can't distinguish from an accidental duplicate id.
    maturity = "development",
    requirements = [":feat_req"],
    tests = [],
    deps = [],
)

# Live example of clickable_plantuml's "public API" linking chain: an
# interface bound to a unit in the *static* architecture (public_api_overview.puml,
# a component diagram) references `PublicInterface`, and public_api_detail.puml
# (also a component-style diagram, passed via architectural_design's
# `public_api` attribute, which also feeds FMEA/safety-analysis traceability -
# mirrors examples/seooc/design/public_api.puml) defines it. Both share the
# FQN `package_pub.PublicInterface`. Regression-checked by
# :public_api_example_link_rendered_test below.
architectural_design(
    name = "arch_design_public_api_example",
    public_api = ["fixtures/clickable_example/public_api_detail.puml"],
    static = ["fixtures/clickable_example/public_api_overview.puml"],
)

# public_api items must be referenced by a FailureMode in the SEooC's own
# safety analysis (unlike static/dynamic/internal_api) - the shared
# dependability_analysis_target/samplelibrary_safety_analysis fixture used by
# other targets in this file is deliberately empty, so this scenario gets its
# own minimal FMEA that traces to "package_pub.PublicInterface".
fmea(
    name = "public_api_example_fmea",
    arch_design = ":arch_design_public_api_example",
    failuremodes = ["fixtures/clickable_example/public_api_failure_modes.trlc"],
    root_causes = ["fixtures/clickable_example/public_api_fta.puml"],
)

dependability_analysis(
    name = "public_api_example_dependability_analysis",
    arch_design = ":arch_design_public_api_example",
    fmea = [":public_api_example_fmea"],
)

dependable_element(
    name = "public_api_example_lib",
    architectural_design = [":arch_design_public_api_example"],
    assumptions_of_use = [":aous"],
    components = [],
    dependability_analysis = [":public_api_example_dependability_analysis"],
    integrity_level = "B",
    # Downgraded to warnings for the same reason as clickable_example_lib
    # above: this fixture intentionally reuses "PublicInterface" as both a
    # reference and a definition, which the dependable-element validator
    # can't distinguish from an accidental duplicate id.
    maturity = "development",
    requirements = [":feat_req"],
    tests = [],
    deps = [],
)

# Live example of clickable_plantuml's "unit to class diagram" linking chain:
# the *static* architecture (unit_overview.puml, a component diagram) shows
# `unit_one` as a leaf unit (no children) - a reference - and
# unit_class_detail.puml (a class diagram naming a class after the same unit,
# nested in the matching namespace) defines it. Both share the FQN
# `package_u.component_u.unit_one`, since component-diagram ids and
# class-diagram namespace FQNs use the same dot-joined scheme. Regression-
# checked by :unit_example_link_rendered_test below.
architectural_design(
    name = "arch_design_unit_example",
    static = [
        "fixtures/clickable_example/unit_overview.puml",
        "fixtures/clickable_example/unit_class_detail.puml",
    ],
)

dependable_element(
    name = "unit_example_lib",
    architectural_design = [":arch_design_unit_example"],
    assumptions_of_use = [":aous"],
    components = [],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    # Downgraded to warnings for the same reason as clickable_example_lib
    # above: this fixture intentionally reuses "unit_one" (well, its FQN) as
    # both a reference and a definition, which the dependable-element
    # validator can't distinguish from an accidental duplicate id.
    maturity = "development",
    requirements = [":feat_req"],
    tests = [],
    deps = [],
)

# Live example of clickable_plantuml's namespace-as-define synthesis: unlike
# :unit_example above (whose class is literally named after the unit),
# namespace_unit_class_detail.puml elaborates `ns_unit` purely via the
# enclosing namespace `package_ns::component_ns::ns_unit`, nesting a
# differently-named class (`RealImpl`) inside it. class_model_to_idmap() must
# synthesize a define for the namespace FQN itself (not just for RealImpl)
# for namespace_unit_overview.puml's leaf `ns_unit` reference to resolve.
# Routed through unit_design()/unit()/component() (not a bare
# architectural_design static list) so this also regression-checks
# unit_design.bzl's idmap generation. Regression-checked by
# :namespace_unit_example_link_rendered_test below.
unit_design(
    name = "namespace_unit_design",
    static = ["fixtures/clickable_example/namespace_unit_class_detail.puml"],
)

unit(
    name = "namespace_unit",
    testonly = True,
    # RealImpl is illustrative only (mock_lib1 has no matching C++ class);
    # downgrade the design-vs-implementation validation finding to a
    # warning, same rationale as maturity="development" elsewhere in this
    # file.
    maturity = "development",
    tests = [":test_unit_tests"],
    unit_design = [":namespace_unit_design"],
    implementation = [":mock_lib1"],
)

component(
    name = "namespace_unit_component",
    testonly = True,
    components = [":namespace_unit"],
    requirements = [
        ":comp_req",
        ":feat_req",
    ],
    tests = [],
)

architectural_design(
    name = "arch_design_namespace_unit_example",
    static = ["fixtures/clickable_example/namespace_unit_overview.puml"],
)

dependable_element(
    name = "namespace_unit_example_lib",
    testonly = True,
    architectural_design = [":arch_design_namespace_unit_example"],
    assumptions_of_use = [":aous"],
    components = [":namespace_unit_component"],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    # Downgraded to warnings for the same reason as clickable_example_lib
    # above: this fixture intentionally reuses "ns_unit" (its FQN) as both a
    # reference (static architecture) and a definition (namespace synthesis),
    # which the dependable-element validator can't distinguish from an
    # accidental duplicate id.
    maturity = "development",
    requirements = [":feat_req"],
    tests = [],
    deps = [],
)

# Regression test: clickable_plantuml must inject a working link between a
# unit shown in the static architecture (namespace_unit_overview.puml) and
# its elaborating class diagram, whose only linkable element is the
# namespace itself, not a same-named class (namespace_unit_class_detail.puml,
# built via unit_design()) - the "namespace-as-define synthesis" scenario.
sh_test(
    name = "namespace_unit_example_link_rendered_test",
    srcs = ["check_idmap_link.sh"],
    args = [
        "package_ns.component_ns.ns_unit",
        "$(rootpaths :namespace_unit_example_lib_index)",
    ],
    data = [":namespace_unit_example_lib_index"],
)

# Live example of clickable_plantuml's "dynamic to static" linking chain: the
# *dynamic* architecture (dynamic_overview.puml, a sequence diagram) shows
# participants `Dispatcher` and `Worker` - both references, since sequence
# diagrams have no defines - and dynamic_component_detail.puml (a component
# diagram in the *static* architecture) shows both as leaf units (also
# references, since neither has children of its own). Both units are
# elaborated by their own class diagrams (dynamic_dispatcher_class.puml,
# dynamic_worker_class.puml), which is where the actual click target lands -
# mirrors :unit_example above, applied to both units of a dynamic scenario.
# TaskInterface (shared by both units) is likewise only a reference in the
# component diagram; its definition lives in the *public API*
# (dynamic_public_api_detail.puml), exactly like :public_api_example above.
# Regression-checked by :dynamic_example_link_rendered_test,
# :dynamic_dispatcher_class_link_rendered_test,
# :dynamic_worker_class_link_rendered_test and
# :dynamic_public_api_link_rendered_test below.
#
# NOTE: clicking on individual *messages* in the sequence diagram (e.g.
# "Assign"/"Done") is not currently supported - sequence_model_to_idmap()
# only tracks participant names, not per-message/method labels (see
# puml_idmap/src/lib.rs). Implementing that would require the sequence
# resolver to persist message identifiers in SequenceTree first.
architectural_design(
    name = "arch_design_dynamic_example",
    dynamic = ["fixtures/clickable_example/dynamic_overview.puml"],
    public_api = ["fixtures/clickable_example/dynamic_public_api_detail.puml"],
    static = [
        "fixtures/clickable_example/dynamic_component_detail.puml",
        "fixtures/clickable_example/dynamic_dispatcher_class.puml",
        "fixtures/clickable_example/dynamic_worker_class.puml",
    ],
)

# public_api items must be referenced by a FailureMode in the SEooC's own
# safety analysis (see :public_api_example_fmea above for the same reasoning)
# - the shared dependability_analysis_target fixture is deliberately empty, so
# this scenario gets its own minimal FMEA that traces to "TaskInterface".
fmea(
    name = "dynamic_example_fmea",
    arch_design = ":arch_design_dynamic_example",
    failuremodes = ["fixtures/clickable_example/dynamic_public_api_failure_modes.trlc"],
    root_causes = ["fixtures/clickable_example/dynamic_public_api_fta.puml"],
)

dependability_analysis(
    name = "dynamic_example_dependability_analysis",
    arch_design = ":arch_design_dynamic_example",
    fmea = [":dynamic_example_fmea"],
)

dependable_element(
    name = "dynamic_example_lib",
    architectural_design = [":arch_design_dynamic_example"],
    assumptions_of_use = [":aous"],
    components = [],
    dependability_analysis = [":dynamic_example_dependability_analysis"],
    integrity_level = "B",
    # Downgraded to warnings for the same reason as clickable_example_lib
    # above: this fixture intentionally reuses "Dispatcher"/"Worker"/
    # "TaskInterface" as both references and definitions, which the
    # dependable-element validator can't distinguish from an accidental
    # duplicate id.
    maturity = "development",
    requirements = [":feat_req"],
    tests = [],
    deps = [],
)

# Minimal dependable element used as a dependency target to test
# the deps attribute of dependable_element (i.e. SEooC depending on another SEooC).
dependable_element(
    name = "dep_seooc_lib",
    architectural_design = [":arch_design_dep"],
    assumptions_of_use = [":aous"],
    components = [],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    requirements = [":feat_req"],
    tests = [],
    deps = [],
)

dependable_element(
    name = "seooc_test_lib",
    architectural_design = [":arch_design"],
    assumptions_of_use = [":aous"],
    components = [],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    requirements = [":feat_req"],
    tests = [],
    deps = [":dep_seooc_lib"],  # dependency to other seoocs/score_components
)

# ============================================================================
# Test Fixtures - Unit, Component, and Dependable Element
# ============================================================================

# Mock implementation targets with dummy functions
cc_library(
    name = "mock_lib1",
    srcs = ["fixtures/mock_lib1.cc"],
)

cc_library(
    name = "mock_lib2",
    srcs = ["fixtures/mock_lib2.cc"],
)

cc_binary(
    name = "test_component_binary",
    srcs = ["fixtures/test_component_main.cc"],
    deps = [
        ":mock_lib1",
        ":mock_lib2",
    ],
)

cc_test(
    name = "test_unit_tests",
    testonly = True,
    srcs = ["fixtures/test_unit_test.cc"],
    tags = ["manual"],
    deps = [
        ":mock_lib1",
        ":mock_lib2",
        "@googletest//:gtest_main",
    ],
)

# Test Unit Design
unit_design(
    name = "test_unit_design",
    dynamic = ["fixtures/seooc_test/dynamic_architecture.rst"],
    static = ["fixtures/seooc_test/static_architecture.rst"],
)

# Test Unit
unit(
    name = "test_unit",
    testonly = True,
    scope = ["//:mock_lib1"],
    tests = [":test_unit_tests"],
    unit_design = [":test_unit_design"],
    implementation = [
        ":mock_lib1",
        ":mock_lib2",
    ],
)

cc_test(
    name = "test_binary_unit_tests",
    testonly = True,
    srcs = ["fixtures/test_binary_unit_test.cc"],
    tags = ["manual"],
    deps = [
        ":mock_lib1",
        ":mock_lib2",
        "@googletest//:gtest_main",
    ],
)

unit(
    name = "test_binary_unit",
    testonly = True,
    tests = [":test_binary_unit_tests"],
    unit_design = [":test_unit_design"],
    implementation = [
        ":mock_lib1",
        ":mock_lib2",
    ],
)

# Test Component
component(
    name = "test_component",
    testonly = True,
    components = [
        ":test_unit",
        "test_binary_unit",
    ],
    requirements = [
        ":comp_req",
        ":feat_req",
    ],
    tests = [],  # Empty for testing
)

# Test Dependable Element.
# maturity = "development": this fixture intentionally omits a
# test_case_coverage.lock.yaml, so under the release traceability gate its
# empty "Test Case Coverage" level would be reported as a missing element.
# Development maturity relaxes that gate.
dependable_element(
    name = "test_dependable_element",
    testonly = True,
    architectural_design = [":arch_design_elem"],
    assumptions_of_use = [":aous"],
    components = [":test_component"],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    maturity = "development",
    requirements = [":feat_req"],
    tests = [],  # Empty for testing
)

# Dependable elements wrapping a test_case_coverage_lock component — used to verify the
# ComponentTestCaseCoverageLockCheck action wiring (mnemonic/inputs/argv) at both
# maturity levels via test_case_coverage_lock_check_action_{release,development}_test.
dependable_element(
    name = "test_dependable_element_coverage_release",
    testonly = True,
    architectural_design = [":arch_design_elem"],
    assumptions_of_use = [":aous"],
    components = [":test_component_with_test_case_coverage_lock"],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    maturity = "release",
    requirements = [":feat_req"],
    tags = ["manual"],
    tests = [],  # Empty for testing
)

dependable_element(
    name = "test_dependable_element_coverage_development",
    testonly = True,
    architectural_design = [":arch_design_elem"],
    assumptions_of_use = [":aous"],
    components = [":test_component_with_test_case_coverage_lock"],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    maturity = "development",
    requirements = [":feat_req"],
    tags = ["manual"],
    tests = [],  # Empty for testing
)

# ============================================================================
# Test Fixtures - Nested Components for Recursive Testing
# ============================================================================

# Additional mock implementations
cc_library(
    name = "mock_lib3",
    srcs = ["fixtures/mock_lib1.cc"],  # Reuse same source for testing
)

cc_test(
    name = "test_unit2_tests",
    testonly = True,
    srcs = ["fixtures/test_unit2_test.cc"],
    tags = ["manual"],
    deps = [
        ":mock_lib2",
        ":mock_lib3",
        "@googletest//:gtest_main",
    ],
)

# Second unit that will be shared between components
unit(
    name = "test_unit2",
    testonly = True,
    tests = [":test_unit2_tests"],
    unit_design = [":test_unit_design"],
    implementation = [":mock_lib3"],
)

# Nested component containing unit2
component(
    name = "test_nested_component",
    testonly = True,
    components = [":test_unit2"],
    requirements = [
        ":comp_req",
        ":feat_req",
    ],
    tests = [],
)

# Parent component containing nested component and another unit
component(
    name = "test_parent_component",
    testonly = True,
    components = [
        ":test_nested_component",
        ":test_unit",  # Different unit in the parent
    ],
    requirements = [
        ":comp_req",
        ":feat_req",
    ],
    tests = [],
)

# Dependable element with nested components to test recursive collection.
# maturity = "development": like ``test_dependable_element`` it has no
# test_case_coverage.lock.yaml, so the release gate would flag a missing
# Test Case Coverage element.
dependable_element(
    name = "test_dependable_element_nested",
    testonly = True,
    architectural_design = [":arch_design_nested"],
    assumptions_of_use = [":aous"],
    components = [":test_parent_component"],
    dependability_analysis = [":dependability_analysis_target"],
    integrity_level = "B",
    maturity = "development",
    requirements = [":feat_req"],
    tests = [],
)

# ============================================================================
# Test Instantiations - HTML Generation Tests
# ============================================================================

# Needs Generation Tests
needs_transitive_test(
    name = "needs_transitive_test",
    target_under_test = ":module_b_lib_needs",
)

# Dependency Tests
module_dependencies_test(
    name = "module_dependencies_test",
    target_under_test = ":module_a_lib",
)

html_merging_test(
    name = "html_merging_test",
    target_under_test = ":module_a_lib",
)

providers_test(
    name = "providers_test",
    target_under_test = ":module_a_lib",
)

basic_html_generation_test(
    name = "basic_html_generation_test",
    target_under_test = ":module_a_lib",
)

needs_generation_test(
    name = "needs_generation_test",
    target_under_test = ":module_b_lib_needs",
)

auto_config_generation_test(
    name = "auto_config_generation_test",
    target_under_test = ":module_c_lib",
)

explicit_config_test(
    name = "explicit_config_test",
    target_under_test = ":module_a_lib",
)

# ============================================================================
# HTML Content Validation Tests
# ============================================================================

# Test that generated HTML does not contain "Unknown need" statements.
# Covers both direct-dep needs resolution (module_a_lib) and the two-hop case
# (module_e_lib) -- see check_unknown_needs.sh's TODO for the hardcoded-path
# cleanup this could still use.
sh_test(
    name = "no_unknown_needs_test",
    srcs = ["check_unknown_needs.sh"],
    data = [
        ":module_a_lib",
        ":module_e_lib",
    ],
    tags = ["manual"],
)

# End-to-end regression test: module_d_lib (a diamond dep of both
# module_b_lib and module_c_lib, never directly of module_a_lib) must be
# merged into module_a_lib's published site exactly once, flat, not once
# per dependency path that reaches it.
sh_test(
    name = "module_d_dedup_test",
    srcs = ["check_module_d_dedup.sh"],
    data = [":module_a_lib"],
    tags = ["manual"],
)

# ============================================================================
# SEooC-Specific Tests
# ============================================================================

# Test that all artifacts are copied
seooc_artifacts_copied_test(
    name = "seooc_tests_artifacts_copied",
    target_under_test = ":seooc_test_lib_index",
)

# Regression test: generated submodule links must point to <dep>_doc/index.html.
sh_test(
    name = "seooc_tests_dep_links_use_doc_dir",
    srcs = ["check_seooc_dep_links.sh"],
    args = ["$(rootpaths :seooc_test_lib_index)"],
    data = [":seooc_test_lib_index"],
)

# Regression test: clickable_plantuml must actually inject a working link
# between the two example diagrams in fixtures/clickable_example/ (mirrors
# //plantuml/sphinx/clickable_plantuml/README.md's documented example).
sh_test(
    name = "clickable_example_link_rendered_test",
    srcs = ["check_clickable_example_link.sh"],
    args = ["$(rootpaths :clickable_example_lib_index)"],
    data = [":clickable_example_lib_index"],
)

# Regression test: clickable_plantuml must inject a working link between the
# "InternalInterface" reference (interface_overview.puml, a component
# diagram) and its definition (interface_detail.puml, auto-detected as a
# class diagram) - the cross-diagram-type "interface" scenario.
sh_test(
    name = "interface_example_link_rendered_test",
    srcs = ["check_idmap_link.sh"],
    args = [
        "package_a.InternalInterface",
        "$(rootpaths :interface_example_lib_index)",
    ],
    data = [":interface_example_lib_index"],
)

# Regression test: clickable_plantuml must inject a working link between the
# "AuditTrail" reference (class_overview.puml) and its definition
# (class_detail.puml) - the pure class-diagram scenario.
sh_test(
    name = "class_example_link_rendered_test",
    srcs = ["check_idmap_link.sh"],
    args = [
        "AuditTrail",
        "$(rootpaths :class_example_lib_index)",
    ],
    data = [":class_example_lib_index"],
)

# Regression test: clickable_plantuml must inject a working link between an
# interface used in the static architecture (public_api_overview.puml) and
# its definition in the public API diagram (public_api_detail.puml) - the
# "interface in static architecture links to public API" scenario.
sh_test(
    name = "public_api_example_link_rendered_test",
    srcs = ["check_idmap_link.sh"],
    args = [
        "package_pub.PublicInterface",
        "$(rootpaths :public_api_example_lib_index)",
    ],
    data = [":public_api_example_lib_index"],
)

# Regression test: clickable_plantuml must inject a working link between a
# unit shown in the static architecture (unit_overview.puml) and its
# elaborating class diagram (unit_class_detail.puml) - the "unit links to its
# class diagram" scenario.
sh_test(
    name = "unit_example_link_rendered_test",
    srcs = ["check_idmap_link.sh"],
    args = [
        "package_u.component_u.unit_one",
        "$(rootpaths :unit_example_lib_index)",
    ],
    data = [":unit_example_lib_index"],
)

# Regression test: clickable_plantuml must inject a working link between a
# participant shown in the dynamic architecture (dynamic_overview.puml, a
# sequence diagram) and the corresponding unit in the static architecture
# (dynamic_component_detail.puml) - the "dynamic participant links to static
# unit" scenario.
sh_test(
    name = "dynamic_example_link_rendered_test",
    srcs = ["check_idmap_link.sh"],
    args = [
        "Worker",
        "$(rootpaths :dynamic_example_lib_index)",
    ],
    data = [":dynamic_example_lib_index"],
)

# Regression test: clickable_plantuml must inject a working link between the
# "Dispatcher" unit (a reference in both dynamic_overview.puml and
# dynamic_component_detail.puml) and its elaborating class diagram
# (dynamic_dispatcher_class.puml) - the "unit links to its own class diagram"
# scenario, for the first of the two units.
sh_test(
    name = "dynamic_dispatcher_class_link_rendered_test",
    srcs = ["check_idmap_link.sh"],
    args = [
        "Dispatcher",
        "$(rootpaths :dynamic_example_lib_index)",
    ],
    data = [":dynamic_example_lib_index"],
)

# Regression test: same as above, for the second unit ("Worker" links to
# dynamic_worker_class.puml).
sh_test(
    name = "dynamic_worker_class_link_rendered_test",
    srcs = ["check_idmap_link.sh"],
    args = [
        "Worker",
        "$(rootpaths :dynamic_example_lib_index)",
    ],
    data = [":dynamic_example_lib_index"],
)

# Regression test: clickable_plantuml must inject a working link between
# "TaskInterface" as used in the static architecture
# (dynamic_component_detail.puml) and its definition in the public API
# (dynamic_public_api_detail.puml) - the "interface in static architecture
# links to public API" scenario, applied to the dynamic_example units.
sh_test(
    name = "dynamic_public_api_link_rendered_test",
    srcs = ["check_idmap_link.sh"],
    args = [
        "TaskInterface",
        "$(rootpaths :dynamic_example_lib_index)",
    ],
    data = [":dynamic_example_lib_index"],
)

# Test that sphinx_module is generated with correct providers
seooc_sphinx_module_generated_test(
    name = "seooc_tests_sphinx_module_generated",
    target_under_test = ":seooc_test_lib",
)

# Test that needs provider exists for cross-referencing
seooc_needs_provider_test(
    name = "seooc_tests_needs_provider",
    target_under_test = ":seooc_test_lib_doc_needs",
)

seooc_index_generation_test(
    name = "seooc_tests_index_generation",
    target_under_test = ":seooc_test_lib_index",
)

# Regression tests: correct index.rst resolution when multiple index.rst files exist
seooc_multi_index_files_exist_test(
    name = "seooc_tests_multi_index_files_exist",
    target_under_test = ":test_dependable_element_index",
)

seooc_sphinx_entry_point_is_root_index_test(
    name = "seooc_tests_sphinx_entry_point_is_root_index",
    target_under_test = ":test_dependable_element_doc",
)

seooc_all_sources_relocated_test(
    name = "seooc_tests_all_sources_relocated",
    target_under_test = ":test_dependable_element_doc",
)

seooc_index_file_provider_test(
    name = "seooc_tests_index_file_provider",
    target_under_test = ":test_dependable_element_index",
)

# ============================================================================
# Test Suites
# ============================================================================

# Main test suite combining all sphinx_module tests
sphinx_module_test_suite(name = "sphinx_module_tests")

# Sphinx module providers test suite
score_needs_with_deps_test(
    name = "score_needs_with_deps_test",
    target_under_test = ":module_a_lib_needs",
)

# Toolchain resolution: this module's own //:score_toolchain must win over
# score_tooling's sphinx_default_toolchain (see MODULE.bazel's
# register_toolchains("//:score_toolchain")).
score_toolchain_override_test(
    name = "score_toolchain_override_test",
    target_under_test = ":module_c_lib",
)

worker_gating_html_test(
    name = "worker_gating_html_test",
    target_under_test = ":module_worker_enabled_lib",
)

worker_gating_needs_test(
    name = "worker_gating_needs_test",
    target_under_test = ":module_worker_enabled_lib_needs",
)

deps_html_merging_test(
    name = "deps_html_merging_test",
    target_under_test = ":module_a_lib",
)

deps_needs_collection_test(
    name = "deps_needs_collection_test",
    target_under_test = ":module_a_lib_needs",
)

sphinx_module_info_fields_test(
    name = "sphinx_module_info_fields_test",
    target_under_test = ":module_a_lib",
)

# Regression test for the diamond-dependency HTML-merge flattening:
# module_d_lib is a dep of both module_b_lib and module_c_lib (never
# directly of module_a_lib), so it's reachable via two paths.
transitive_modules_dedup_test(
    name = "transitive_modules_dedup_test",
    target_under_test = ":module_a_lib",
)

score_needs_info_fields_test(
    name = "score_needs_info_fields_test",
    target_under_test = ":module_a_lib_needs",
)

score_needs_transitive_collection_test(
    name = "score_needs_transitive_collection_test",
    target_under_test = ":module_a_lib_needs",
)

two_phase_needs_first_test(
    name = "two_phase_needs_first_test",
    target_under_test = ":module_b_lib_needs",
)

two_phase_html_second_test(
    name = "two_phase_html_second_test",
    target_under_test = ":module_a_lib",
)

config_auto_generation_test(
    name = "config_auto_generation_test",
    target_under_test = ":module_c_lib",
)

config_explicit_usage_test(
    name = "config_explicit_usage_test",
    target_under_test = ":module_a_lib",
)

sphinx_module_providers_test_suite(name = "sphinx_module_providers_tests")

# SEooC-focused test suite
test_suite(
    name = "seooc_tests",
    tests = [
        ":seooc_tests_all_sources_relocated",
        ":seooc_tests_artifacts_copied",
        ":seooc_tests_dep_links_use_doc_dir",
        ":seooc_tests_index_file_provider",
        ":seooc_tests_index_generation",
        ":seooc_tests_multi_index_files_exist",
        ":seooc_tests_needs_provider",
        ":seooc_tests_sphinx_entry_point_is_root_index",
        ":seooc_tests_sphinx_module_generated",
    ],
)

# ============================================================================
# Unit, Component, and Dependable Element Test Instantiations
# ============================================================================

# Unit tests
unit_provider_test(
    name = "unit_provider_test",
    target_under_test = ":test_unit",
)

unit_sphinx_sources_test(
    name = "unit_sphinx_sources_test",
    target_under_test = ":test_unit",
)

# Test Component with test case coverage lock — verifies analysis phase succeeds when
# test_case_coverage_lock is provided.
component(
    name = "test_component_with_test_case_coverage_lock",
    testonly = True,
    components = [":test_unit"],
    requirements = [":comp_req"],
    tags = ["manual"],
    test_case_coverage_lock = "fixtures/test_case_coverage_lock_test/test_case_coverage.lock.yaml",
    tests = [],
)

# Component tests
component_provider_test(
    name = "component_provider_test",
    target_under_test = ":test_component",
)

component_sphinx_sources_test(
    name = "component_sphinx_sources_test",
    target_under_test = ":test_component",
)

component_excludes_feature_req_docs_test(
    name = "component_excludes_feature_req_docs_test",
    target_under_test = ":test_component",
)

component_test_case_coverage_lock_test(
    name = "component_test_case_coverage_lock_test",
    target_under_test = ":test_component_with_test_case_coverage_lock",
)

# Verifies the ComponentTestCaseCoverageLockCheck action is wired up end-to-end through
# dependable_element (not just component() in isolation), and that
# --allow-check-failures is only passed for maturity == "development".
dependable_element_test_case_coverage_lock_check_action_test(
    name = "test_case_coverage_lock_check_action_release_test",
    expect_allow_check_failures = False,
    maturity_under_test = "release",
    target_under_test = ":test_dependable_element_coverage_release_index",
)

dependable_element_test_case_coverage_lock_check_action_test(
    name = "test_case_coverage_lock_check_action_development_test",
    expect_allow_check_failures = True,
    maturity_under_test = "development",
    target_under_test = ":test_dependable_element_coverage_development_index",
)

# Unit, Component, and Dependable Element test suite
unit_component_test_suite(name = "unit_component_tests")

# ============================================================================
# RST-based Requirements Tests
# ============================================================================

# Fixture: assumed_system_requirements from RST
assumed_system_requirements(
    name = "asr_rst",
    srcs = ["fixtures/rst_requirements/assumed_system_requirements.rst"],
)

# Fixture: feature_requirements from RST (depends on asr_rst for derived_from resolution)
feature_requirements(
    name = "feat_req_rst",
    srcs = ["fixtures/rst_requirements/feature_requirements.rst"],
    ref_package = "AssumedSystemRequirements",
    deps = [":asr_rst"],
)

# Fixture: component_requirements from RST
component_requirements(
    name = "comp_req_rst",
    srcs = ["fixtures/rst_requirements/component_requirements.rst"],
    ref_package = "FeatureRequirements",
    deps = [
        ":asr_rst",
        ":feat_req_rst",
    ],
)

# Fixture: assumptions_of_use from RST
assumptions_of_use(
    name = "aous_rst",
    srcs = ["fixtures/rst_requirements/assumptions_of_use.rst"],
)

asr_rst_output_test(
    name = "asr_rst_output_test",
    target_under_test = ":asr_rst",
)

asr_rst_provider_test(
    name = "asr_rst_provider_test",
    target_under_test = ":asr_rst",
)

asr_rst_sphinx_test(
    name = "asr_rst_sphinx_test",
    target_under_test = ":asr_rst",
)

feat_req_rst_provider_test(
    name = "feat_req_rst_provider_test",
    target_under_test = ":feat_req_rst",
)

feat_req_rst_sphinx_test(
    name = "feat_req_rst_sphinx_test",
    target_under_test = ":feat_req_rst",
)

comp_req_rst_provider_test(
    name = "comp_req_rst_provider_test",
    target_under_test = ":comp_req_rst",
)

comp_req_rst_sphinx_test(
    name = "comp_req_rst_sphinx_test",
    target_under_test = ":comp_req_rst",
)

aous_rst_provider_test(
    name = "aous_rst_provider_test",
    target_under_test = ":aous_rst",
)

aous_rst_sphinx_test(
    name = "aous_rst_sphinx_test",
    target_under_test = ":aous_rst",
)

requirements_rst_test_suite(name = "requirements_rst_tests")

# ============================================================================
# lobster_config.bzl unit tests (format_lobster_sources / format_lobster_block)
# ============================================================================
lobster_config_test_suite(name = "lobster_config")

# ============================================================================
# Image srcs Tests (defined in sub-package to avoid workspace-root edge case)
# ============================================================================
# feat_req_with_image and image_srcs_sphinx_sources_test live in
# //fixtures/image_srcs so that ctx.label.package is non-empty and
# subrule_trlc_image_stage can strip the prefix correctly.

# ============================================================================
# Combined Test Suite
# ============================================================================

# ============================================================================
# Safety Analysis Tools Unit Tests
# ============================================================================

# Validates the updated Measure type hierarchy (ControlMeasure, PreventiveMeasure,
# Mitigation, AoU) introduced in the tracing-policy update (commit bff44df3):
# ControlMeasure no longer carries a "mitigates" field; tracing is implicit via
# name-matching with the FTA $BasicEvent alias.
trlc_requirements(
    name = "safety_measures_fixtures",
    srcs = ["fixtures/seooc_test/safety_measures_fixtures.trlc"],
    spec = ["@score_tooling//bazel/rules/rules_score/trlc/config:score_requirements_model"],
)

trlc_requirements_test(
    name = "safety_measures_types_test",
    reqs = [":safety_measures_fixtures"],
)

py_test(
    name = "test_fmea_assembler",
    size = "small",
    srcs = ["test_fmea_assembler.py"],
    deps = ["@score_tooling//bazel/rules/rules_score:fmea_assembler"],
)

py_test(
    name = "test_aou_forwarding_to_lobster",
    size = "small",
    srcs = ["test_aou_forwarding_to_lobster.py"],
    deps = ["@score_tooling//bazel/rules/rules_score:aou_forwarding_to_lobster"],
)

py_test(
    name = "test_sphinx_html_merge",
    size = "small",
    srcs = ["test_sphinx_html_merge.py"],
    deps = ["@score_tooling//bazel/rules/rules_score:sphinx_html_merge_lib"],
)

py_test(
    name = "test_bazel_sphinx_needs",
    size = "small",
    srcs = ["test_bazel_sphinx_needs.py"],
    deps = ["@score_tooling//bazel/rules/rules_score:bazel_sphinx_needs"],
)

py_test(
    name = "test_sphinx_module_ext",
    size = "small",
    srcs = ["test_sphinx_module_ext.py"],
    deps = ["@score_tooling//bazel/rules/rules_score:sphinx_module_ext"],
)

py_test(
    name = "test_rst_to_trlc",
    size = "small",
    srcs = ["rst_to_trlc_test.py"],
    main = "rst_to_trlc_test.py",
    deps = ["@score_tooling//bazel/rules/rules_score:rst_to_trlc_lib"],
)

py_test(
    name = "test_trlc_rst_image_rendering",
    size = "small",
    srcs = ["trlc_rst_image_rendering_test.py"],
    data = [
        "fixtures/image_requirements/image_requirements.trlc",
        "fixtures/image_requirements/schema.rsl",
    ],
    main = "trlc_rst_image_rendering_test.py",
    deps = ["@trlc//tools/trlc_rst:trlc_rst_lib"],
)

# Combined test suite for all tests
test_suite(
    name = "all_tests",
    tests = [
        ":lobster_config_tests",
        ":requirements_rst_tests",
        ":seooc_tests",
        ":sphinx_module_tests",
        ":test_aou_forwarding_to_lobster",
        ":test_bazel_sphinx_needs",
        ":test_fmea_assembler",
        ":test_rst_to_trlc",
        ":test_sphinx_html_merge",
        ":test_sphinx_module_ext",
        ":test_trlc_rst_image_rendering",
        ":transitive_modules_dedup_test",
        ":unit_component_tests",
        "//fixtures/image_srcs:requirements_image_tests",
    ],
)
