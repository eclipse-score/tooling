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
from typing import List

from python.runfiles import Runfiles
from sphinx.util import logging

# Create a logger with the Sphinx namespace
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Helpers: Bazel execroot path resolution
# ---------------------------------------------------------------------------


# Resolve execroot-relative paths against the Bazel execroot.  sphinx_wrapper.py
# exports SPHINX_BAZEL_EXECROOT (the action cwd captured before Sphinx changes
# into the generated source tree).  Fall back to the current cwd for non-Bazel
# / IDE runs where the variable is absent.
_EXECROOT = Path(os.environ.get("SPHINX_BAZEL_EXECROOT", "") or Path.cwd())


def _resolve_execroot_path(path_value: str) -> str:
    """Resolve an execroot-relative path to an absolute filesystem path.

    Bazel passes action inputs as paths relative to the execroot (e.g.
    ``external/+_repo_rules+graphviz_deb/usr/bin/dot_builtins``).  Sphinx changes
    into the generated source tree before importing conf.py, so the process cwd
    is no longer the execroot.  We resolve against ``_EXECROOT`` (captured by the
    wrapper before that chdir) so the paths stay valid for the ``dot``
    subprocess.

    Absolute paths and plain command names (e.g. ``dot``) are returned
    unchanged.
    """
    p = Path(path_value)
    if p.is_absolute():
        return str(p)
    if path_value.startswith("external/") or path_value.startswith("bazel-out/"):
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


# Use the runfiles to find the plantuml binary.
# Runfiles are only available when running in Bazel.
r = Runfiles.Create()
if r is None:
    raise ValueError("Could not initialize Bazel runfiles.")

plantuml_repo_candidates: List[str] = []

for repo_name in [
    os.environ.get("TEST_WORKSPACE"),
    "_main",
    "score_tooling",
    "score_tooling~",
    "score_tooling+",
]:
    if repo_name and repo_name not in plantuml_repo_candidates:
        plantuml_repo_candidates.append(repo_name)

plantuml_runfiles_candidates = [
    f"{repo_name}/tools/sphinx/plantuml" for repo_name in plantuml_repo_candidates
]

plantuml_path = None
for runfile_path in plantuml_runfiles_candidates:
    candidate = r.Rlocation(runfile_path, source_repo="")
    if candidate and Path(candidate).exists():
        plantuml_path = Path(candidate)
        logger.info(f"Selected PlantUML from runfiles path {runfile_path}: {candidate}")
        break

if plantuml_path is None:
    searched = ", ".join(plantuml_runfiles_candidates)
    raise ValueError(
        f"Could not find plantuml binary via runfiles lookup. Searched: {searched}."
    )

# ---------------------------------------------------------------------------
# PlantUML + hermetic dot
# ---------------------------------------------------------------------------
# GRAPHVIZ_DOT is set by sphinx_module on linux_x86_64 to the hermetic
# dot_builtins binary from @graphviz_deb.  When present, PlantUML is told to
# use it directly via -graphvizdot, giving native Graphviz layout quality for
# all UML diagram types.  LD_LIBRARY_PATH / LTDL_LIBRARY_PATH are resolved to
# absolute paths here so they remain valid in the dot_builtins subprocess that
# PlantUML spawns (Sphinx may have chdir'd before then).
# On other platforms (e.g. arm64, macOS) GRAPHVIZ_DOT is absent and PlantUML
# falls back to its built-in Smetana layout engine (pure-Java, no dot needed).
if "GRAPHVIZ_DOT" in os.environ:
    _dot_path = Path(_resolve_execroot_path(os.environ["GRAPHVIZ_DOT"]))
    # Derive library search paths from the binary location so the rule passes
    # only GRAPHVIZ_DOT and conf.py stays self-contained.
    # The graphviz cmake deb installs:
    #   usr/bin/dot_builtins       ← GRAPHVIZ_DOT points here
    #   usr/lib/*.so*              ← LD_LIBRARY_PATH (core shared libs)
    #   usr/lib/graphviz/*.so*     ← LTDL_LIBRARY_PATH (layout/render plugins)
    _usr_dir = _dot_path.parent.parent  # usr/bin → parent → usr
    os.environ["LD_LIBRARY_PATH"] = str(_usr_dir / "lib")
    os.environ["LTDL_LIBRARY_PATH"] = str(_usr_dir / "lib" / "graphviz")
    plantuml = f"{plantuml_path} -graphvizdot {_dot_path}"
else:
    logger.warning(
        "GRAPHVIZ_DOT not set; PlantUML falling back to Smetana layout engine. "
        "Hermetic dot (@graphviz_deb) is only available on linux_x86_64."
    )
    plantuml = f"{plantuml_path} -Playout=smetana"
plantuml_output_format = "svg_obj"

# HTML theme
html_theme = "sphinx_rtd_theme"

# Note: version_flyout.css and version_flyout.js are injected by the
# deploy workflow via _shared/ paths so they load once across all versions.

logger.debug("#" * 80)
