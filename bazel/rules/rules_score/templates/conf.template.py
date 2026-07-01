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
# PLANTUML_BIN     — execroot-relative path of //third_party/plantuml:plantuml
#                    (a rules_java java_binary launcher script), injected by the
#                    sphinx_module Bazel rule via the action env.
# PLANTUML_BIN_RLOC — analysis-time-stable Bazel rlocation key derived from the
#                    target's short_path (no exec-config hash); used only for
#                    diagnostic logging here.
#
# Path resolution rationale (applies to all hermetic tool paths in this file):
# os.path.abspath() converts the execroot-relative path to an absolute path
# using the process's current working directory.  Bazel guarantees that the
# action's cwd equals the execroot at process start.  conf.py is loaded during
# Sphinx initialisation — before Sphinx can perform any os.chdir() — so the
# abspath() call is stable for the entire action lifetime.  This replaces the
# previous _resolve_execroot_path() which walked parent directories as a
# fallback, a pattern that is fragile and wrong when nested under bazel-out/.
# See docs/tooling_architecture.rst §"Hermetic tool path resolution".
_plantuml_bin = os.environ.get("PLANTUML_BIN")
if not _plantuml_bin:
    raise ValueError(
        "PLANTUML_BIN environment variable is not set. It must point at the "
        "//third_party/plantuml:plantuml launcher and is normally provided by the "
        "sphinx_module Bazel rule. If you are invoking Sphinx outside that rule, "
        "set PLANTUML_BIN to the plantuml binary path."
    )
plantuml_path = os.path.abspath(_plantuml_bin)
logger.debug(
    f"plantuml resolved: {plantuml_path} "
    f"(rloc: {os.environ.get('PLANTUML_BIN_RLOC', 'n/a')})"
)

plantuml_output_format = "svg_obj"
# `plantuml` command is assembled below, after graphviz_dot is resolved, so
# PlantUML can use the same hermetic Graphviz dot (see Graphviz section).

# ---------------------------------------------------------------------------
# Graphviz (sphinx.ext.graphviz)
# ---------------------------------------------------------------------------
# GRAPHVIZ_DOT      — execroot-relative path of //third_party/docs_runtime:dot
#                     (the exec_in_sysroot POSIX-sh wrapper), injected by the
#                     sphinx_module Bazel rule.
# GRAPHVIZ_DOT_RLOC — analysis-time-stable rlocation key; logged only.
#
# Path resolution: same os.path.abspath() rationale as PLANTUML_BIN above.
#
# The exec_in_sysroot wrapper is a runfiles-aware POSIX-sh script that
# bootstraps its own runfiles via the standard $0.runfiles/ Bazel convention.
# Passing the ABSOLUTE path ensures $0 is absolute, so $0.runfiles/ resolves to
# the correct companion directory even when the wrapper is called as a
# subprocess from inside the Sphinx Python process (which carries its own
# RUNFILES_DIR pointing at the sphinx tool's runfiles, not the dot wrapper's
# runfiles).
#
# GRAPHVIZ_DOT is mandatory: the sphinx_module rule always provides the hermetic
# wrapper, so if it is missing conf.py fails loudly rather than silently using a
# host-installed dot binary.
_graphviz_dot_path = os.environ.get("GRAPHVIZ_DOT")
if not _graphviz_dot_path:
    raise ValueError(
        "GRAPHVIZ_DOT environment variable is not set. It must point at the "
        "//third_party/docs_runtime:dot hermetic wrapper and is normally provided "
        "by the sphinx_module Bazel rule. If you are invoking Sphinx outside that "
        "rule, set GRAPHVIZ_DOT to the hermetic dot wrapper path."
    )
_graphviz_dot_rloc = os.environ.get("GRAPHVIZ_DOT_RLOC", "")
graphviz_dot = os.path.abspath(_graphviz_dot_path)
graphviz_output_format = "svg"
logger.debug(
    f"graphviz dot resolved: {graphviz_dot} (rloc: {_graphviz_dot_rloc or 'n/a'})"
)

# ---------------------------------------------------------------------------
# PlantUML layout engine: hermetic dot + FTA metamodel include path
# ---------------------------------------------------------------------------
# FTA_METAMODEL_DIR — directory containing fta_metamodel.puml, set by the
#                     sphinx_module rule from //plantuml:fta_metamodel.
#                     FTA diagrams keep ``!include fta_metamodel.puml``;
#                     sphinxcontrib-plantuml renders via -pipe (no source-file
#                     dir on the include search path) so the file must be
#                     listed on plantuml.include.path.  The JVM flag must
#                     precede the program args or the java_binary launcher
#                     passes it to PlantUML instead of the JVM.
_fta_metamodel_dir = os.environ.get("FTA_METAMODEL_DIR", "")
if _fta_metamodel_dir:
    _fta_metamodel_dir = os.path.abspath(_fta_metamodel_dir)
    logger.debug(f"fta_metamodel include path: {_fta_metamodel_dir}")
    _include_flag = f" --jvm_flag=-Dplantuml.include.path={_fta_metamodel_dir}"
else:
    logger.warning(
        "FTA_METAMODEL_DIR is not set; FTA diagrams using "
        "!include fta_metamodel.puml will fail to render."
    )
    _include_flag = ""

# PlantUML uses the same hermetic Graphviz dot as sphinx.ext.graphviz for its
# internal layout calls, via the -graphvizdot flag.  There is no fallback: the
# hermetic dot is the single reference rendering path for both.
plantuml = f"{plantuml_path}{_include_flag} -graphvizdot {graphviz_dot}"

# HTML theme
html_theme = "sphinx_rtd_theme"

# Note: version_flyout.css and version_flyout.js are injected by the
# deploy workflow via _shared/ paths so they load once across all versions.

logger.debug("#" * 80)
