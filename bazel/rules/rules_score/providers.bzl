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

SphinxSourcesInfo = provider(
    doc = """Provider for Sphinx documentation source files.

    This provider aggregates all source files needed for Sphinx documentation
    builds, including reStructuredText, Markdown, PlantUML diagrams, and
    image files. Rules that produce documentation artifacts should provide
    this to enable integration with sphinx_module and dependable_element.
    """,
    fields = {
        "srcs": "Depset of source files for Sphinx documentation (.rst, .md, .puml, .plantuml, .svg, .png, etc.)",
        "transitive_srcs": "Depset of transitive source files from dependencies",
    },
)

UnitInfo = provider(
    doc = "Provider for unit artifacts",
    fields = {
        "name": "Name of the unit target",
        "unit_design": "Depset of unit design artifacts (architectural design)",
        "implementation": "Depset of implementation targets (libraries, binaries)",
        "tests": "Depset of test targets",
    },
)

ComponentInfo = provider(
    doc = "Provider for component artifacts",
    fields = {
        "name": "Name of the component target",
        "requirements": "Depset of component requirements artifacts",
        "components": "Depset of unit targets that comprise this component",
        "tests": "Depset of component-level integration test targets",
    },
)

UnitDesignInfo = provider(
    doc = """Provider for unit design artifacts.

    Carries parsed representations of static and dynamic design views for a
    software unit. The depset fields are populated once a diagram parser is
    integrated; until then they are empty stubs.
    """,
    fields = {
        "static": "Depset of parsed static design view artifacts (e.g., class diagrams)",
        "dynamic": "Depset of parsed dynamic design view artifacts (e.g., sequence diagrams)",
        "name": "Name of the unit design target",
    },
)

ArchitecturalDesignInfo = provider(
    doc = "Provider for architectural design artifacts",
    fields = {
        "static": "Depset of static architecture diagram files (e.g., class diagrams, component diagrams)",
        "dynamic": "Depset of dynamic architecture diagram files (e.g., sequence diagrams, activity diagrams)",
        "name": "Name of the architectural design target",
    },
)

AnalysisInfo = provider(
    doc = "Provider for FMEA and safety analysis artifacts",
    fields = {
        "controlmeasures": "Depset of control measures documentation or requirements",
        "failuremodes": "Depset of failure modes documentation or requirements",
        "fta": "Depset of Fault Tree Analysis diagrams",
        "arch_design": "ArchitecturalDesignInfo provider for linked architectural design",
        "name": "Name of the analysis target",
    },
)

DependabilityAnalysisInfo = provider(
    doc = "Provider for dependability analysis artifacts",
    fields = {
        "safety_analysis": "List of AnalysisInfo providers",
        "security_analysis": "List of AnalysisInfo providers",
        "arch_design": "ArchitecturalDesignInfo provider for linked architectural design",
        "name": "Name of the dependability analysis target",
    },
)

FeatureRequirementsInfo = provider(
    doc = "Provider for feature requirements artifacts",
    fields = {
        "srcs": "Depset of source files containing feature requirements",
        "name": "Name of the feature requirements target",
    },
)

ComponentRequirementsInfo = provider(
    doc = "Provider for component requirements artifacts",
    fields = {
        "srcs": "Depset of source files containing component requirements",
        "requirements": "List of FeatureRequirementsInfo providers this component traces to",
        "name": "Name of the component requirements target",
    },
)

AssumptionsOfUseInfo = provider(
    doc = "Provider for assumptions of use artifacts",
    fields = {
        "srcs": "Depset of source files containing assumptions of use",
        "feature_requirements": "List of FeatureRequirementsInfo providers this AoU traces to",
        "name": "Name of the assumptions of use target",
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
