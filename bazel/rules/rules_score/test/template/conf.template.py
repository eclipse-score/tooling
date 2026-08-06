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

import bazel_sphinx_needs
import sphinx_conf_helpers

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
# The actual `graphviz_dot`/`plantuml` config values are NOT set here: this
# module-level scope runs inside Sphinx's chdir(confdir), so resolving
# PLANTUML_BIN/GRAPHVIZ_DOT's execroot-relative paths via os.path.abspath()
# here would resolve against the wrong base. setup() below instead connects
# sphinx_conf_helpers.init_hermetic_tools as a "config-inited" listener,
# which fires once cwd is back at the execroot -- mirroring the production
# bazel/rules/rules_score/templates/conf.template.py, which uses the same
# function via the sphinx_module_ext extension.
#
# The hermetic dot sysroot has its GD plugin pruned (no X11/pango/GD deps in
# the minimal sysroot), so it does not support png output. Sphinx's built-in
# graphviz extension (used e.g. by lobster's ".. graphviz:: tracing_policy"
# diagrams) defaults to png; force svg, which the hermetic dot supports.
graphviz_output_format = "svg"

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
    # Sphinx invokes conf.py's own setup() as a bare `self.config.setup(self)`
    # statement (sphinx.application.Sphinx._init_builder) -- its return value
    # is never read, so both calls below are plain statements rather than one
    # being `return`ed while the other's registration is discarded.
    bazel_sphinx_needs.setup_sphinx_extension(app, needs_external_needs)
    app.connect("config-inited", sphinx_conf_helpers.init_hermetic_tools)
