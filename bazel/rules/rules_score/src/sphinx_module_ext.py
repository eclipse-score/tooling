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
#
"""Sphinx extension entry point for loading external (cross-module) needs
and resolving hermetic tool paths.

Registered by listing "sphinx_module_ext" in conf.py's `extensions = [...]`;
Sphinx then auto-invokes `setup(app)` below, no manual wiring required. This
is the counterpart to bazel_sphinx_needs.py, which offers the same
find_workspace_root()/load_external_needs() logic for conf.py authors who
prefer to import and wire it up explicitly instead of registering an
extension. See bazel_sphinx_needs.py's module docstring for that alternative.
"""

from pathlib import Path
import re
from typing import Any, Dict

from sphinx.errors import NoUri

from bazel_sphinx_needs import load_external_needs
from sphinx_conf_helpers import init_hermetic_tools

# Field-list marker emitted by rules_score's generated directory-navigation
# index.rst pages (see puml_utils.bzl's emit_view_navigation). A leading
# field list is parsed by docutils/Sphinx as document metadata rather than
# rendered content, landing in app.env.metadata[docname] -- the same
# mechanism Sphinx itself uses for ":orphan:".
_DIRECTORY_INDEX_METADATA_KEY = "score-directory-index"

# Cache key for the marked-docname set computed once per build and reused on
# every html-page-context call (one per output page), rather than rescanning
# app.env.metadata (all documents) for every page. The compiled regex itself
# is *not* cached across pages: it embeds relative URIs, which depend on the
# current page's own location in the doc tree. Cached on `app` (not `app.env`)
# because BuildEnvironment is pickled to environment.pickle for incremental
# rebuilds and `app` is not, so a stale set can never be restored from disk.
_DIRECTORY_INDEX_DOCNAMES_ATTR = "score_directory_index_docnames"


def init_external_needs(app: Any, config: Any) -> None:
    """
    Initialize external needs configuration.

    "config-inited" fires with cwd == execroot, not confdir -- Sphinx's
    chdir(confdir) only wraps evaluating conf.py itself, and that context
    has already exited by the time this listener runs. needs_external_needs.json
    lives beside conf.py in confdir, so it must be looked up explicitly.

    Args:
        app: Sphinx application object
        config: Sphinx configuration object
    """

    config.needs_external_needs = load_external_needs(Path(app.confdir))


def _directory_index_docnames(app: Any) -> set:
    """Return docnames of generated directory-navigation index pages,
    computing and caching the set on ``app`` the first time it's needed so
    repeated html-page-context calls -- one per output page -- don't rescan
    ``app.env.metadata`` (all documents) for every page.

    Identified precisely via the ``score-directory-index`` metadata marker
    rather than by pattern-matching rendered URLs/filenames, so genuine
    hand-authored pages that happen to be named index.rst are never
    affected.
    """
    docnames = getattr(app, _DIRECTORY_INDEX_DOCNAMES_ATTR, None)
    if docnames is None:
        docnames = {docname for docname, meta in app.env.metadata.items() if _DIRECTORY_INDEX_METADATA_KEY in meta}
        setattr(app, _DIRECTORY_INDEX_DOCNAMES_ATTR, docnames)
    return docnames


def _directory_index_anchor_pattern(app: Any, pagename: str) -> "re.Pattern | None":
    """Build the anchor-matching regex for this page's rendered sidebar.

    Relative URIs (and hence the regex) depend on `pagename`'s own location
    in the doc tree, so this is rebuilt per page from the cached docname set
    -- only the (page-independent, and usually far more expensive) metadata
    scan in `_directory_index_docnames` is cached across pages.

    Returns None if there are no generated directory-index pages to relink
    to from this page.
    """
    docnames = _directory_index_docnames(app)
    if not docnames:
        return None

    urls = set()
    for docname in docnames:
        try:
            urls.add(app.builder.get_relative_uri(pagename, docname))
        except NoUri:
            # No URL can be built from this page to that docname (e.g. it
            # was excluded from this build) -- nothing to relink.
            continue
    if not urls:
        return None

    return re.compile(
        r'<a(?P<before>[^>]*?)href="(?P<href>'
        + "|".join(re.escape(url) for url in urls)
        + r')"(?P<after>[^>]*)>(?P<label>.*?)</a>',
        re.DOTALL,
    )


def render_directory_labels_without_links(
    app: Any,
    pagename: str,
    templatename: str,
    context: Dict[str, Any],
    doctree: Any,
) -> None:
    """Remove navigation links for generated directory index pages.

    Directory indexes exist only to provide expandable navigation groups. The
    sidebar should expose their names as labels, while diagram pages remain
    normal links.

    Requires a conf.py that keeps the sidebar expanded (see conf.template.py's
    html_theme_options): a de-linked group header can no longer be clicked to
    expand, so a collapsing theme would hide its diagram pages entirely.
    """
    anchor_pattern = _directory_index_anchor_pattern(app, pagename)
    if anchor_pattern is None:
        return

    def _to_span(match: "re.Match") -> str:
        attrs = (match.group("before") + match.group("after")).strip()
        return "<span{}>{}</span>".format(" " + attrs if attrs else "", match.group("label"))

    def wrap_toctree_renderer(renderer: Any) -> Any:
        def render_without_directory_links(*args: Any, **kwargs: Any) -> str:
            html = renderer(*args, **kwargs)
            return anchor_pattern.sub(_to_span, str(html))

        return render_without_directory_links

    # sphinx_rtd_theme renders via "toctree"; pydata_sphinx_theme's sidebar
    # renders via "generate_toctree_html" instead -- wrap whichever is present.
    for renderer_name in ("toctree", "generate_toctree_html"):
        renderer = context.get(renderer_name)
        if renderer is not None:
            context[renderer_name] = wrap_toctree_renderer(renderer)


def setup(app: Any) -> Dict[str, Any]:
    """
    Sphinx setup hook to register event listeners.

    Args:
        app: Sphinx application object

    Returns:
        Extension metadata dictionary
    """
    app.connect("config-inited", init_external_needs)
    app.connect("config-inited", init_hermetic_tools)
    app.connect("html-page-context", render_directory_labels_without_links, priority=900)

    return {
        "version": "1.0",
        "parallel_read_safe": True,
        "parallel_write_safe": True,
    }
