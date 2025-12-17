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
        "implementation": "Depset of implementation targets (libraries, binaries)",
        "components": "Depset of unit targets that comprise this component",
        "tests": "Depset of component-level integration test targets",
    },
)
