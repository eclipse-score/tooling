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
from pathlib import Path

from sphinx.util import logging

# Create a logger with the Sphinx namespace
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Helpers: Bazel execroot path resolution
# ---------------------------------------------------------------------------


# Capture the current working directory at module import time.
# In Bazel action context, cwd == execroot. In IDE/non-Bazel runs, cwd is
# the current directory. This is captured once to avoid repeated resolution.
_EXECROOT = Path.cwd()


def _resolve_execroot_path(path_value: str) -> str:
    """Resolve an execroot-relative path to an absolute filesystem path.

    Bazel passes action inputs as paths relative to the execroot (e.g.
    ``bazel-bin/third_party/docs_runtime/dot``).  Those paths are only valid
    when the process' cwd is the execroot — which is not guaranteed once
    Sphinx changes directories during the build.

    This function makes them absolute so they work regardless of cwd.
    Absolute paths and plain command names (e.g. ``dot``) are returned
    unchanged.
    """
    p = Path(path_value)
    if p.is_absolute():
        return str(p)
    if path_value.startswith("external/") or path_value.startswith("bazel-out/"):
        # First try cwd-as-execroot (fast path).
        candidate = (_EXECROOT / p).resolve()
        if candidate.exists():
            return str(candidate)

        # If cwd is nested under bazel-out, walk upward and locate the first
        # parent that contains the requested relative path.
        for parent in [_EXECROOT, *_EXECROOT.parents]:
            candidate = (parent / p).resolve()
            if candidate.exists():
                return str(candidate)

        # Fallback: preserve previous behavior even if the file does not exist
        # yet (keeps logging/debug output deterministic).
        return str((_EXECROOT / p).resolve())
    return path_value


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
project_url = (
    "https://github.com/eclipse-score"  # Required by score_metamodel extension
)

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
myst_enable_extensions = ["colon_fence"]

# Exclude patterns for Bazel builds
exclude_patterns = [
    "bazel-*",
    ".venv*",
    # Design-fragment subdirectories (e.g. units/unit_1_design/) are included
    # via '.. include::' directives and must not be treated as standalone pages.
    "**/*_design",
]

# Suppress toctree warnings for documents absent from the needs builder's source
# tree.  The needs builder runs against only the static docs/ checkout; generated
# files (trlc_rst outputs, renamed_srcs, docs_library_deps) live in bazel-out/
# and are invisible to it.  Their toctree references produce toc.not_readable
# warnings that are cosmetic: the needs builder (sphinx-needs NeedsBuilder)
# captures only `.. need::` directives, not trlc `.. requirement:definition::`
# directives, so needs.json content is unaffected by missing files.
# This suppression is safe for the HTML phase because that phase relocates every
# file into a unified staging directory, so it never encounters toc.not_readable.
suppress_warnings = ["toc.not_readable"]

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
# Instead, we call load_metamodel_data() directly from yaml_parser — the
# score_docs_as_code+ repo root IS on sys.path, so the import resolves — and
# we get only the type/option/regex data without activating the validation hooks.
try:
    from src.extensions.score_metamodel.yaml_parser import (
        load_metamodel_data as _load_metamodel_data,
    )

    _metamodel = _load_metamodel_data()
    needs_types = _metamodel.needs_types
    needs_extra_options = _metamodel.needs_extra_options
    needs_extra_links = _metamodel.needs_extra_links
    needs_id_regex = "^[A-Za-z0-9_-]{6,}"
except ImportError:
    logger.warning("score_metamodel not available; using minimal needs_types fallback")
    needs_types = []
    needs_extra_options = []
    needs_extra_links = []
    needs_id_regex = "^[A-Za-z0-9_-]{6,}"


# ---------------------------------------------------------------------------
# PlantUML binary discovery
# ---------------------------------------------------------------------------
# PLANTUML_BIN is set by the Bazel sphinx_module rule to the explicit execroot-
# relative path of the //third_party/plantuml:plantuml java_binary launcher script.
_plantuml_bin = os.environ.get("PLANTUML_BIN")
if not _plantuml_bin:
    raise ValueError(
        "PLANTUML_BIN environment variable is not set. It must point at the "
        "//third_party/plantuml:plantuml launcher and is normally provided by the "
        "sphinx_module Bazel rule. If you are invoking Sphinx outside that rule, "
        "set PLANTUML_BIN to the plantuml binary path."
    )
plantuml_path = _resolve_execroot_path(_plantuml_bin)

plantuml_output_format = "svg_obj"
# `plantuml` is defined below, after graphviz_dot is resolved, so PlantUML can
# render with the same hermetic Graphviz dot (see end of the Graphviz section).

# ---------------------------------------------------------------------------
# Graphviz (sphinx.ext.graphviz)
# ---------------------------------------------------------------------------
# GRAPHVIZ_DOT is set by the Bazel sphinx_module rule to point at the
# exec_in_sysroot wrapper from //third_party/docs_runtime:dot.
# The wrapper executes /usr/bin/dot inside the hermetic docs_runtime sysroot.
# Paths are passed execroot-relative, so resolve to absolute for robustness if
# Sphinx changes cwd before spawning dot.
# If GRAPHVIZ_DOT is absent, force a known-invalid dot path so Sphinx fails
# clearly on graphviz directives instead of silently using host-installed dot.
if "GRAPHVIZ_DOT" in os.environ:
    graphviz_dot = _resolve_execroot_path(os.environ["GRAPHVIZ_DOT"])
    graphviz_output_format = "svg"
else:
    graphviz_dot = "/__hermetic_graphviz_not_configured__/dot"

# Render PlantUML diagrams with the real (hermetic) Graphviz dot resolved above,
# giving reference Graphviz layout. Smetana (PlantUML's bundled Java port) was
# previously used only to avoid an external dot in the sandbox; the hermetic dot
# removes that constraint. Swap back to `-Playout=smetana` if the per-diagram dot
# subprocess regresses build time.
plantuml = f"{plantuml_path} -graphvizdot {graphviz_dot}"

# HTML theme
html_theme = "sphinx_rtd_theme"

# Note: version_flyout.css and version_flyout.js are injected by the
# deploy workflow via _shared/ paths so they load once across all versions.

logger.debug("#" * 80)
