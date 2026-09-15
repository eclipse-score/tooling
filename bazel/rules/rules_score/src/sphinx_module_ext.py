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

from bazel_sphinx_needs import load_external_needs
from sphinx_conf_helpers import init_hermetic_tools


_DIRECTORY_INDEX_ANCHOR = re.compile(
    r'<a(?P<before>[^>]*?)href="[^"]+/index\.html"(?P<after>[^>]*)>(?P<label>.*?)</a>',
    re.DOTALL,
)


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
    """

    def wrap_toctree_renderer(renderer: Any) -> Any:
        def render_without_directory_links(*args: Any, **kwargs: Any) -> str:
            kind = args[0] if args else kwargs.get("kind")
            if kind == "sidebar":
                kwargs["show_nav_level"] = 100
                kwargs["maxdepth"] = 100
            html = renderer(*args, **kwargs)
            return _DIRECTORY_INDEX_ANCHOR.sub(
                lambda match: "<span{}{}>{}</span>".format(
                    match.group("before"),
                    match.group("after"),
                    match.group("label"),
                ),
                str(html),
            )

        return render_without_directory_links

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
