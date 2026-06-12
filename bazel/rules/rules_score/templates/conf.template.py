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
import shutil as _shutil
import sys
from pathlib import Path
from typing import Any, Dict, List

from python.runfiles import Runfiles
from sphinx.util import logging

# Create a logger with the Sphinx namespace
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Helpers: Bazel execroot path resolution
# ---------------------------------------------------------------------------


def _bazel_execroot() -> Path:
    """Return the Bazel execroot directory inferred from this config file's path.

    conf.py is generated into ``bazel-out/…/bin/…/conf.py``, so splitting on
    ``/bazel-out/`` gives us the execroot prefix reliably.  Falls back to the
    current working directory when the path pattern is not recognised (e.g.
    during unit tests or IDE runs outside Bazel).
    """
    parts = str(Path(__file__).resolve()).split("/bazel-out/", 1)
    return Path(parts[0]) if len(parts) == 2 else Path.cwd()


# Computed once at import time so _resolve_execroot_path() doesn't repeat the
# filesystem resolution on every call.
_EXECROOT = _bazel_execroot()


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
graphviz_dot = _resolve_execroot_path(
    os.environ.get("GRAPHVIZ_DOT") or _shutil.which("dot") or "dot"
)

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

# HTML theme
html_theme = "sphinx_rtd_theme"

# Note: version_flyout.css and version_flyout.js are injected by the
# deploy workflow via _shared/ paths so they load once across all versions.

logger.debug("#" * 80)
