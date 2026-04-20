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
Shared providers for S-CORE documentation build rules.

This module defines providers that are shared across multiple documentation
build rules to enable consistent Sphinx documentation generation.
"""

# ============================================================================
# Provider Definitions
# ============================================================================

CertifiedScope = provider(
    doc = """Holds the scope labels that are certified by this target.

    This provider aggregates the list of labels which are under the
    certification scope of this target.

    Valid values are either:
    * an existing label
    * a package (e.g. //some/package:__pkg__)
    * a package and its subpackages (e.g. //some/package:__subpackages__)

    This follows the Bazel visibility patterns.
    Note that broad visibility labels (//visibility:public or //visibility:private)
    do not make sense for this kind of scoping and are thus not functional.
    """,
    fields = {
        "transitive_scopes": "Depset of Labels (packages or targets) that are under the certification scope of this target, collected transitively.",
    },
)

SphinxSourcesInfo = provider(
    doc = """Provider for Sphinx documentation source files.

    This provider aggregates all source files needed for Sphinx documentation
    builds, including reStructuredText, Markdown, PlantUML diagrams, and
    image files. Rules that produce documentation artifacts should provide
    this to enable integration with sphinx_module and dependable_element.
    """,
    fields = {
        "srcs": "Depset of direct source files for Sphinx documentation (.rst, .md, .puml, .plantuml, .svg, .png, etc.)",
        "deps": "Depset of transitive Sphinx source files collected from all direct and transitive dependencies.",
    },
)

UnitInfo = provider(
    doc = "Provider for unit artifacts.",
    fields = {
        "name": "Name of the unit target.",
        "unit_design": "Depset of design artifact files (PlantUML diagrams, RST documents, etc.).",
        "unit_design_static_fbs": "Depset of FlatBuffers binaries generated from static unit_design diagrams.",
        "unit_design_dynamic_fbs": "Depset of FlatBuffers binaries generated from dynamic unit_design diagrams.",
        "implementation": "Depset of implementation targets (cc_library, rust_library, etc.).",
        "tests": "Depset of test targets (cc_test, rust_test, etc.).",
        "dependent_labels": "Depset of Labels that this unit's implementation depends on transitively (used for certification scope validation).",
    },
)

FeatureRequirementsInfo = provider(
    doc = "Provider for feature requirements artifacts.",
    fields = {
        "srcs": "Depset of .lobster traceability files generated from TRLC requirement sources.",
        "name": "Name of the requirements target.",
    },
)

ComponentRequirementsInfo = provider(
    doc = "Provider for component requirements artifacts.",
    fields = {
        "srcs": "Depset of .lobster traceability files generated from TRLC requirement sources.",
        "name": "Name of the requirements target.",
    },
)

AnalysisInfo = provider(
    doc = "Provider for safety analysis traceability artifacts (lobster files).",
    fields = {
        "name": "Name of the analysis target.",
        "lobster_files": "Dict mapping canonical lobster file names to File objects " +
                         "(e.g. {'failuremodes.lobster': File, 'root_causes.lobster': File}).",
    },
)

AssumptionsOfUseInfo = provider(
    doc = "Provider for assumptions of use artifacts.",
    fields = {
        "srcs": "Depset of .lobster traceability files collected from all linked requirements targets.",
        "requirements": "List of FeatureRequirementsInfo or ComponentRequirementsInfo providers this AoU traces to.",
        "name": "Name of the assumptions of use target.",
    },
)

ComponentInfo = provider(
    doc = "Provider for component artifacts.",
    fields = {
        "name": "Name of the component target.",
        "requirements": "Depset of requirement traceability files (.lobster) collected from requirements targets, including transitive files from nested components.",
        "components": "Depset of nested component and/or unit Targets that comprise this component.",
        "tests": "Depset of test traceability files (.lobster) generated from unit test results, collected transitively from all nested components and units.",
        "architecture": "Depset of architecture traceability files (.lobster) generated from unit architectural designs, collected transitively from all nested components and units.",
        "dependent_labels": "Depset of Labels that this component's implementation depends on transitively (collected from all nested units and components, used for certification scope validation).",
    },
)

CcDependencyInfo = provider(
    doc = """Provider for collecting transitive dependencies from C/C++ targets.

    This provider aggregates all Labels that a cc_library or cc_binary
    target depends on transitively.
    """,
    fields = {
        "dependencies": "Depset of Labels representing all transitive C/C++ dependencies of a target.",
    },
)

DependableElementInfo = provider(
    doc = """Provider for dependable element metadata.

    Carries the integrity level of a dependable element so that consumers
    (e.g. other dependable elements that list this one in their `deps`)
    can perform integrity-level compatibility checks.

    Allowed values for `integrity_level` are "A", "B", "C", "D" where
    D > C > B > A (D being the highest / most stringent).
    """,
    fields = {
        "integrity_level": "String – one of 'A', 'B', 'C', 'D'.",
        "label": "Label of the dependable element that produced this provider.",
    },
)

ArchitecturalDesignInfo = provider(
    doc = "Provider for architectural design artifacts including parsed design metadata.",
    fields = {
        "static": "Depset of FlatBuffers binaries for static architecture diagrams (class diagrams, component diagrams, etc.)",
        "dynamic": "Depset of FlatBuffers binaries for dynamic architecture diagrams (sequence diagrams, activity diagrams, etc.)",
        "name": "Name of the architectural design target",
        "lobster_files": "Depset of .lobster traceability files generated by the PlantUML parser from component diagrams.",
        "public_api_lobster_files": "Depset of .lobster traceability files generated from public_api diagrams (subset of lobster_files).",
    },
)

UnitDesignInfo = provider(
    doc = "Provider for unit design artifacts including parsed design metadata.",
    fields = {
        "static": "Depset of FlatBuffers binaries for static unit design diagrams (class diagrams, etc.)",
        "dynamic": "Depset of FlatBuffers binaries for dynamic unit design diagrams (sequence diagrams, etc.)",
        "name": "Name of the unit design target",
    },
)

DependabilityAnalysisInfo = provider(
    doc = """Provider for dependability analysis artifacts.

    Aggregates sub-analyses:
      * **fmea**              – fmea rule targets (FM + CM + optional root causes).
      * **security_analysis** – security analysis targets (placeholder).
    """,
    fields = {
        "fmea": "Depset of output files from fmea targets.",
        "security_analysis": "Depset of output files from security analysis targets.",
        "dfa": "Depset of DFA documentation files (placeholder).",
        "arch_design": "ArchitecturalDesignInfo from the linked architectural design (placeholder).",
        "name": "Name of the dependability analysis target.",
        "lobster_files": "Dict mapping canonical lobster file names to File objects collected from sub-analyses only (FM, CM, RC). Does not include architecture lobster files; those are obtained separately via ArchitecturalDesignInfo.lobster_files.",
    },
)

DependableElementLobsterInfo = provider(
    doc = """Provider carrying the lobster traceability report produced by a dependable element.

    Exposed so the main dependable_element test rule can pick up the already-built
    report and wire it into the test executable without running lobster a second time.
    """,
    fields = {
        "lobster_report": "The lobster report JSON File object, or None when no traceability data is available.",
        "lobster_html_report": "The lobster HTML report File object, or None when no traceability data is available.",
    },
)

SphinxModuleInfo = provider(
    doc = "Provider for Sphinx HTML module documentation",
    fields = {
        "html_dir": "Directory containing HTML files",
    },
)

SphinxNeedsInfo = provider(
    doc = "Provider for sphinx-needs info",
    fields = {
        "needs_json_file": "Direct needs.json file for this module",
        "needs_json_files": "Depset of needs.json files including transitive dependencies",
    },
)
