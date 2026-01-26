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
    this to enable integration with sphinx_module and score_component.
    """,
    fields = {
        "srcs": "Depset of source files for Sphinx documentation (.rst, .md, .puml, .plantuml, .svg, .png, etc.)",
        "transitive_srcs": "Depset of transitive source files from dependencies",
    },
)
