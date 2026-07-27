# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
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
"""Shared conf.py building blocks for score_tooling Sphinx consumers.

Every module/repo that supplies its own `conf_template` to
`score_sphinx_toolchain()` (rather than using score_tooling's default one)
currently has to re-derive several non-obvious pieces of Bazel/Sphinx
plumbing from scratch. This module consolidates them so a custom conf.py can
import it and only set what it actually wants to customize (extensions,
theme, etc.):

- Hermetic PlantUML / Graphviz / FTA-metamodel resolution. The env vars this
  reads (PLANTUML_BIN, GRAPHVIZ_DOT, FTA_METAMODEL_DIR) are set
  unconditionally by `_hermetic_tool_env()` in sphinx_module.bzl for every
  SphinxNeedsBuild/SphinxHtmlBuild action, regardless of which toolchain or
  conf_template is in effect - so this works for any consumer without extra
  Bazel wiring. See docs/tooling_architecture.rst
  §"Hermetic tool path resolution".
- sphinx-needs external-needs loading, re-exported from bazel_sphinx_needs
  rather than re-derived (see that module's docstring for the JSON format).
- The sphinx-needs type/option/link schema loaded from the upstream S-CORE
  metamodel, so custom conf.py files can't silently drift from it.
- A few small shared constants (exclude patterns, suppressed warnings, MyST
  extensions) that encode otherwise-easy-to-miss Bazel-sandbox behavior.
"""

import logging
import os
from typing import Any, Dict, List, Optional

from bazel_sphinx_needs import (
    load_external_needs,
    log_config_info,
    setup_sphinx_extension,
)

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Shared constants
# ---------------------------------------------------------------------------

# Bazel-sandbox artifacts that must never be treated as doc sources.
DEFAULT_EXCLUDE_PATTERNS: List[str] = [
    "bazel-*",
    ".venv*",
    # Design-fragment subdirectories (e.g. units/unit_1_design/) are included
    # via '.. include::' directives and must not be treated as standalone
    # pages.
    "**/*_design",
]

# The needs builder phase runs against only the static docs/ checkout;
# generated files (trlc_rst outputs, renamed_srcs, docs_library_deps) live in
# bazel-out/ and are invisible to it, producing cosmetic toc.not_readable
# warnings. Safe to suppress: the needs builder (sphinx-needs NeedsBuilder)
# only captures `.. need::` directives, so needs.json content is unaffected
# by missing toctree targets. The HTML phase never hits this warning, since
# it relocates every file into one staging directory first.
DEFAULT_SUPPRESS_WARNINGS: List[str] = ["toc.not_readable"]

DEFAULT_MYST_ENABLE_EXTENSIONS: List[str] = ["colon_fence"]


# ---------------------------------------------------------------------------
# Hermetic tool resolution: PlantUML, Graphviz, FTA metamodel include path
# ---------------------------------------------------------------------------


def resolve_graphviz_dot(required: bool = True) -> Optional[str]:
    """Resolve the hermetic Graphviz `dot` wrapper path from GRAPHVIZ_DOT.

    Args:
        required: If True (default), raise ValueError when GRAPHVIZ_DOT is
            unset - matches the default template's fail-loud contract, so a
            missing hermetic tool is a build error, not a silently degraded
            diagram. Pass False only if the caller has a real fallback
            rendering path and genuinely wants to continue without it.
    """
    path = os.environ.get("GRAPHVIZ_DOT")
    if not path:
        msg = (
            "GRAPHVIZ_DOT environment variable is not set. It must point at "
            "the //third_party/docs_runtime:dot hermetic wrapper and is "
            "normally provided by the sphinx_module Bazel rule. If you are "
            "invoking Sphinx outside that rule, set GRAPHVIZ_DOT to the "
            "hermetic dot wrapper path."
        )
        if required:
            raise ValueError(msg)
        logger.warning(msg)
        return None
    resolved = os.path.abspath(path)
    logger.debug(
        "graphviz dot resolved: %s (rloc: %s)",
        resolved,
        os.environ.get("GRAPHVIZ_DOT_RLOC", "n/a"),
    )
    return resolved


def resolve_fta_metamodel_dir() -> str:
    """Resolve the directory containing fta_metamodel.puml from
    FTA_METAMODEL_DIR, or "" (with a warning) if it isn't set.
    """
    raw = os.environ.get("FTA_METAMODEL_DIR", "")
    if not raw:
        logger.warning(
            "FTA_METAMODEL_DIR is not set; FTA diagrams using "
            "!include fta_metamodel.puml will fail to render."
        )
        return ""
    resolved = os.path.abspath(raw)
    logger.debug("fta_metamodel include path: %s", resolved)
    return resolved


def resolve_plantuml_command(
    required: bool = True, graphviz_dot_path: Optional[str] = None
) -> Optional[str]:
    """Build the full `plantuml` conf.py setting.

    Combines PLANTUML_BIN with the FTA metamodel include path and the
    hermetic Graphviz dot, exactly matching the default template's
    configuration. Pair this with `plantuml_output_format = "svg_obj"` in
    conf.py (a fixed literal, not tool-path dependent, so it isn't derived
    here).

    Args:
        required: See `resolve_graphviz_dot`. Also governs whether a missing
            PLANTUML_BIN raises or warns-and-returns-None.
        graphviz_dot_path: If the caller already resolved GRAPHVIZ_DOT (e.g.
            to also set conf.py's own `graphviz_dot` for sphinx.ext.graphviz),
            pass it here to reuse that value instead of re-resolving (and
            re-logging) it. If None (default), resolves it internally.
    """
    plantuml_bin = os.environ.get("PLANTUML_BIN")
    if not plantuml_bin:
        msg = (
            "PLANTUML_BIN environment variable is not set. It must point at "
            "the //third_party/plantuml:plantuml launcher and is normally "
            "provided by the sphinx_module Bazel rule. If you are invoking "
            "Sphinx outside that rule, set PLANTUML_BIN to the plantuml "
            "binary path."
        )
        if required:
            raise ValueError(msg)
        logger.warning(msg)
        return None

    plantuml_path = os.path.abspath(plantuml_bin)
    logger.debug(
        "plantuml resolved: %s (rloc: %s)",
        plantuml_path,
        os.environ.get("PLANTUML_BIN_RLOC", "n/a"),
    )

    fta_dir = resolve_fta_metamodel_dir()
    include_flag = (
        " --jvm_flag=-Dplantuml.include.path=%s" % fta_dir if fta_dir else ""
    )

    dot_path = (
        graphviz_dot_path
        if graphviz_dot_path is not None
        else resolve_graphviz_dot(required=required)
    )
    layout_flag = " -graphvizdot %s" % dot_path if dot_path else ""

    return "%s%s%s" % (plantuml_path, include_flag, layout_flag)


# ---------------------------------------------------------------------------
# sphinx-needs schema, loaded from the upstream S-CORE metamodel
# ---------------------------------------------------------------------------


def load_metamodel_needs_schema() -> Dict[str, Any]:
    """Load the needs_types/needs_extra_options/needs_extra_links/
    needs_id_regex schema from score_metamodel, so a custom conf.py can't
    silently drift from the upstream S-CORE process description.

    score_metamodel is intentionally NOT loaded as a Sphinx extension here
    (i.e. via `extensions = [..., "score_metamodel"]`) - doing so registers
    validation hooks (mandatory options, prohibited words, link pattern
    checks) that do bare `from score_metamodel import ...` imports requiring
    src/extensions/ on sys.path, which is only set up by aspect_rules_py's
    venv mechanism and not guaranteed here. Calling
    load_metamodel_data() directly instead gets only the type/option/regex
    data, without activating those hooks.

    Returns a dict with keys `needs_types`, `needs_extra_options`,
    `needs_extra_links`, `needs_id_regex` - assign each directly in conf.py:

        _schema = load_metamodel_needs_schema()
        needs_types = _schema["needs_types"]
        needs_extra_options = _schema["needs_extra_options"]
        needs_extra_links = _schema["needs_extra_links"]
        needs_id_regex = _schema["needs_id_regex"]

    Falls back to an empty schema (with a warning) if score_metamodel isn't
    a dependency of the calling conf.py's toolchain binary.
    """
    fallback_id_regex = "^[A-Za-z0-9_-]{6,}"
    try:
        from src.extensions.score_metamodel.yaml_parser import (
            load_metamodel_data as _load_metamodel_data,
        )

        metamodel = _load_metamodel_data()
        return {
            "needs_types": metamodel.needs_types,
            "needs_extra_options": metamodel.needs_extra_options,
            "needs_extra_links": metamodel.needs_extra_links,
            "needs_id_regex": fallback_id_regex,
        }
    except ImportError:
        logger.warning(
            "score_metamodel not available; using minimal needs_types fallback"
        )
        return {
            "needs_types": [],
            "needs_extra_options": [],
            "needs_extra_links": [],
            "needs_id_regex": fallback_id_regex,
        }


__all__ = [
    "DEFAULT_EXCLUDE_PATTERNS",
    "DEFAULT_SUPPRESS_WARNINGS",
    "DEFAULT_MYST_ENABLE_EXTENSIONS",
    "resolve_graphviz_dot",
    "resolve_fta_metamodel_dir",
    "resolve_plantuml_command",
    "load_metamodel_needs_schema",
    # Re-exported so consumers only need one import for both needs-loading
    # and hermetic-tool concerns.
    "load_external_needs",
    "log_config_info",
    "setup_sphinx_extension",
]
