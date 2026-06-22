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


# Capture the current working directory at module import time.
# In Bazel action context, cwd == execroot. In IDE/non-Bazel runs, cwd is
# the current directory. This is captured once to avoid repeated resolution.
_EXECROOT = Path.cwd()


def _resolve_execroot_path(path_value: str) -> str:
    """Resolve an execroot-relative path to an absolute filesystem path.

    Bazel passes action inputs as paths relative to the execroot (e.g.
    ``external/+_repo_rules2+graphviz_deb/usr/bin/dot_builtins``).  Those
    paths are only valid when the process' cwd is the execroot — which is
    not guaranteed once Sphinx changes directories during the build.

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

# Use PlantUML's built-in Smetana layout engine (Java port of Graphviz).
# This avoids requiring an external dot binary in the Bazel sandbox.
plantuml = f"{plantuml_path} -Playout=smetana"
plantuml_output_format = "svg_obj"

# ---------------------------------------------------------------------------
# Graphviz (sphinx.ext.graphviz)
# ---------------------------------------------------------------------------
# GRAPHVIZ_DOT is set by the Bazel sphinx_module rule to point at the hermetic
# dot_builtins binary from @graphviz_deb.  The path is execroot-relative, so
# we resolve it to an absolute path here so it remains valid after any cwd
# change that Sphinx may perform during the build.
# If GRAPHVIZ_DOT is absent, force a known-invalid dot path so Sphinx fails
# clearly on graphviz directives instead of silently using host-installed dot.
if "GRAPHVIZ_DOT" in os.environ:
    graphviz_dot = _resolve_execroot_path(os.environ["GRAPHVIZ_DOT"])
    graphviz_output_format = "svg"

    # LD_LIBRARY_PATH and LTDL_LIBRARY_PATH are set by the Bazel rule as
    # execroot-relative paths.  We mutate os.environ (not just a local) because
    # sphinx.ext.graphviz spawns `dot` as a child process that inherits these
    # variables to locate the bundled shared libraries and plugins.  Each
    # component is resolved to absolute so it stays valid if Sphinx changes cwd
    # before spawning the dot subprocess.
    for _env_var in ("LD_LIBRARY_PATH", "LTDL_LIBRARY_PATH"):
        _env_val = os.environ.get(_env_var, "")
        if _env_val:
            os.environ[_env_var] = ":".join(
                _resolve_execroot_path(p) for p in _env_val.split(":")
            )
else:
    graphviz_dot = "/__hermetic_graphviz_not_configured__/dot"

# HTML theme
html_theme = "sphinx_rtd_theme"

# Note: version_flyout.css and version_flyout.js are injected by the
# deploy workflow via _shared/ paths so they load once across all versions.

logger.debug("#" * 80)
