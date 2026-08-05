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
Generic Sphinx configuration template for SCORE modules.

This file is auto-generated from a template and should not be edited directly.
Template variables like {PROJECT_NAME} are replaced during Bazel build.
"""

import json
import os
import sys

from sphinx.util import logging

import sphinx_conf_helpers

# Create a logger with the Sphinx namespace
logger = logging.getLogger(__name__)


logger.debug("#" * 80)
logger.debug("# READING CONF.PY")
logger.debug("SYSPATH:" + str(sys.path))
logger.debug("EMV:" + str(os.environ))

for k, v in os.environ.items():
    logger.debug(str(k) + ": " + v)
# Project configuration - {PROJECT_NAME} will be replaced by the module name during build
project = "{PROJECT_NAME}"
author = "S-CORE"
version = "1.0"
release = "1.0.0"
project_url = "https://github.com/eclipse-score"  # Required by score_metamodel extension

# Sphinx extensions - comprehensive list for SCORE modules
extensions = [
    "sphinx_module_ext",
    "sphinx_needs",
    "sphinx_design",
    "myst_parser",
    "sphinxcontrib.plantuml",
    "trlc",
    "clickable_plantuml",
    "sphinx.ext.graphviz",
]

# MyST parser extensions
myst_enable_extensions = sphinx_conf_helpers.DEFAULT_MYST_ENABLE_EXTENSIONS

# Exclude patterns for Bazel builds. Design-fragment subdirectories (e.g.
# units/unit_1_design/) are included via '.. include::' directives and must
# not be treated as standalone pages -- see DEFAULT_EXCLUDE_PATTERNS.
exclude_patterns = sphinx_conf_helpers.DEFAULT_EXCLUDE_PATTERNS

# Suppress toctree warnings for documents absent from the needs builder's source
# tree.  The needs builder runs against only the static docs/ checkout; generated
# files (trlc_rst outputs, renamed_srcs, docs_library_deps) live in bazel-out/
# and are invisible to it.  Their toctree references produce toc.not_readable
# warnings that are cosmetic: the needs builder (sphinx-needs NeedsBuilder)
# captures only `.. need::` directives, not trlc `.. requirement:definition::`
# directives, so needs.json content is unaffected by missing files.
# Scoped to the needs builder only ({BUILDER} is substituted by
# sphinx_module.bzl) -- the HTML phase relocates every file into a unified
# staging directory, so a toc.not_readable warning there is a genuinely broken
# toctree entry, not a cosmetic one, and must not be suppressed.
suppress_warnings = sphinx_conf_helpers.suppress_warnings_for_builder("{BUILDER}")

# Enable markdown rendering
source_suffix = {
    ".rst": "restructuredtext",
    ".md": "markdown",
}

# Enable numref for cross-references
numfig = True

# sphinx-needs configuration loaded from the upstream S-CORE metamodel.
# The needs types, extra options, extra links and ID regex are derived
# from score_docs_as_code//src/extensions/score_metamodel:metamodel.yaml
# so they stay in sync with the upstream process description.
#
# Note: score_metamodel is NOT loaded as a Sphinx extension
# (i.e. extensions = [..., "score_metamodel"]) for the following reason:
# When loaded as an extension, score_metamodel registers a build-finished hook
# that runs needs validation via its checks/ modules (mandatory options,
# prohibited words, link pattern checks, etc.). Those check modules do
# bare "from score_metamodel import ..." imports, which require src/extensions/
# to be on sys.path. That path is only set up by aspect_rules_py's venv
# mechanism, not by the rules_python setup used here.
# Instead, sphinx_conf_helpers calls load_metamodel_data() directly from
# yaml_parser — the score_docs_as_code+ repo root IS on sys.path, so the
# import resolves — and we get only the type/option/regex data without
# activating the validation hooks.
_needs_schema = sphinx_conf_helpers.load_metamodel_needs_schema()
needs_types = _needs_schema["needs_types"]
needs_extra_options = _needs_schema["needs_extra_options"]
needs_extra_links = _needs_schema["needs_extra_links"]
needs_id_regex = _needs_schema["needs_id_regex"]


# ---------------------------------------------------------------------------
# Hermetic PlantUML / Graphviz / FTA metamodel tool resolution
# ---------------------------------------------------------------------------
# PLANTUML_BIN, GRAPHVIZ_DOT and FTA_METAMODEL_DIR are injected by the
# sphinx_module Bazel rule via the action env (see _hermetic_tool_env() in
# sphinx_module.bzl). Resolution (path rationale, hermeticity requirements,
# the FTA include-path JVM flag, etc.) is centralised in sphinx_conf_helpers
# so every conf.py -- this default template and any custom conf_template --
# shares one implementation. See docs/tooling_architecture.rst
# §"Hermetic tool path resolution".
graphviz_dot = sphinx_conf_helpers.resolve_graphviz_dot()
graphviz_output_format = "svg"

plantuml_output_format = "svg_obj"
# Reuses the graphviz_dot already resolved above instead of re-resolving
# GRAPHVIZ_DOT a second time.
plantuml = sphinx_conf_helpers.resolve_plantuml_command(graphviz_dot_path=graphviz_dot)

# HTML theme
html_theme = "sphinx_rtd_theme"

# Note: version_flyout.css and version_flyout.js are injected by the
# deploy workflow via _shared/ paths so they load once across all versions.

logger.debug("#" * 80)
