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

import os

import bazel_sphinx_needs
from sphinx.util import logging

logger = logging.getLogger(__name__)

# Project configuration - {PROJECT_NAME} will be replaced by the module name during build
project = "{PROJECT_NAME}"
author = "S-CORE"
version = "1.0"
release = "1.0.0"
project_url = "https://github.com/eclipse-score"  # Required by score_metamodel extension

# Sphinx extensions - comprehensive list for SCORE modules
extensions = [
    "sphinx_needs",
    "sphinx_design",
    "myst_parser",
    "sphinxcontrib.plantuml",
    "score_metamodel",
    "score_draw_uml_funcs",
    "score_layout",
    "clickable_plantuml",
]

# ---------------------------------------------------------------------------
# PlantUML / Graphviz binary discovery
# ---------------------------------------------------------------------------
# NOTE: we intentionally do NOT list "score_plantuml" (from score_docs_as_code)
# as an extension here. Its setup() overwrites app.config.plantuml with a path
# derived from RUNFILES_DIR that can be relative to the execroot. sphinxcontrib
# .plantuml invokes the plantuml command via subprocess.Popen(..., cwd=<some
# other dir>) (e.g. the doctree's source dir or its own cache dir), so a
# relative path silently fails to resolve there ('plantuml command ... cannot
# be run'), even though the binary itself works fine when invoked directly.
#
# Instead we resolve PLANTUML_BIN / GRAPHVIZ_DOT — which the sphinx_module
# Bazel rule always injects as action env vars (from //third_party/plantuml
# and //third_party/docs_runtime:dot) — and turn them into ABSOLUTE paths via
# os.path.abspath(). This mirrors the production
# bazel/rules/rules_score/templates/conf.template.py and is immune to any cwd
# changes made by sphinxcontrib-plantuml's subprocess calls.
_plantuml_bin = os.environ.get("PLANTUML_BIN")
if not _plantuml_bin:
    raise ValueError(
        "PLANTUML_BIN environment variable is not set. It must point at the "
        "//third_party/plantuml:plantuml launcher and is normally provided by the "
        "sphinx_module Bazel rule. If you are invoking Sphinx outside that rule, "
        "set PLANTUML_BIN to the plantuml binary path."
    )
plantuml_path = os.path.abspath(_plantuml_bin)

_graphviz_dot_path = os.environ.get("GRAPHVIZ_DOT")
if not _graphviz_dot_path:
    raise ValueError(
        "GRAPHVIZ_DOT environment variable is not set. It must point at the "
        "//third_party/docs_runtime:dot hermetic wrapper and is normally provided "
        "by the sphinx_module Bazel rule. If you are invoking Sphinx outside that "
        "rule, set GRAPHVIZ_DOT to the hermetic dot wrapper path."
    )
graphviz_dot = os.path.abspath(_graphviz_dot_path)

# The hermetic dot sysroot has its GD plugin pruned (no X11/pango/GD deps in
# the minimal sysroot), so it does not support png output. Sphinx's built-in
# graphviz extension (used e.g. by lobster's ".. graphviz:: tracing_policy"
# diagrams) defaults to png; force svg, which the hermetic dot supports.
graphviz_output_format = "svg"

_fta_metamodel_dir = os.environ.get("FTA_METAMODEL_DIR", "")
if _fta_metamodel_dir:
    _fta_metamodel_dir = os.path.abspath(_fta_metamodel_dir)
    _include_flag = f" --jvm_flag=-Dplantuml.include.path={_fta_metamodel_dir}"
else:
    logger.warning("FTA_METAMODEL_DIR is not set; FTA diagrams using !include fta_metamodel.puml will fail to render.")
    _include_flag = ""

# PlantUML uses the same hermetic Graphviz dot as sphinx.ext.graphviz for its
# internal layout calls, via the -graphvizdot flag.
plantuml = f"{plantuml_path}{_include_flag} -graphvizdot {graphviz_dot}"

# Render PlantUML diagrams as SVG so clickable_plantuml's injected links are
# actually clickable in the generated docs (png mode has no clickable overlay).
plantuml_output_format = "svg_obj"

# MyST parser extensions
myst_enable_extensions = ["colon_fence"]

# Exclude patterns for Bazel builds
exclude_patterns = [
    "bazel-*",
    ".venv*",
]

# Enable markdown rendering
source_suffix = {
    ".rst": "restructuredtext",
    ".md": "markdown",
}

# Enable numref for cross-references
numfig = True

# HTML theme
html_theme = "sphinx_rtd_theme"

# Load external needs and log configuration
needs_external_needs = bazel_sphinx_needs.load_external_needs()
bazel_sphinx_needs.log_config_info(project)


def setup(app):
    return bazel_sphinx_needs.setup_sphinx_extension(app, needs_external_needs)
