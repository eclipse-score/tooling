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
from typing import Any, Dict, List

from python.runfiles import Runfiles
from sphinx.util import logging

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

plantuml = str(plantuml_path)
plantuml_output_format = "svg_obj"

# HTML theme
html_theme = "sphinx_rtd_theme"
logger.debug("#" * 80)
