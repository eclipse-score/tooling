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
"""Sphinx extension to make PlantUML diagrams clickable."""

import functools
import json
import re
from pathlib import Path
from typing import Any

from docutils import nodes
from sphinx.application import Sphinx
from sphinx.util import logging

logger = logging.getLogger(__name__)

# Environment attribute names used by this extension.
_ENV_LINK_DATA = "clickable_plantuml_link_data"
# Stores {puml_basename: (docname, anchor_id_or_None)}
_ENV_PUML_DOCNAMES = "clickable_plantuml_puml_docnames"

# Characters allowed in PlantUML alias identifiers.
_ALIAS_SAFE_RE = re.compile(r"^[\w.]+$")


def _find_parent_section_id(node: nodes.Node) -> str | None:
    """Return the HTML anchor of the closest ancestor section node, if any."""
    parent = node.parent
    while parent is not None:
        if isinstance(parent, nodes.section):
            ids: list[str] = parent.get("ids", [])
            if ids:
                return ids[0]
        parent = getattr(parent, "parent", None)
    return None


# ---------------------------------------------------------------------------
# JSON loading
# ---------------------------------------------------------------------------


def _load_link_mappings(
    search_dir: str,
    pattern: str = "*plantuml_links.json",
) -> dict[str, dict[str, Any]]:
    """Return ``{source_file: {source_id: {target_file, ...}}}``."""
    link_data: dict[str, dict[str, Any]] = {}
    for json_file in Path(search_dir).rglob(pattern):
        try:
            json_data = json.loads(json_file.read_text(encoding="utf-8"))
            if "links" not in json_data or not isinstance(json_data["links"], list):
                logger.warning(
                    "Invalid format in %s: missing 'links' array",
                    json_file.name,
                )
                continue
            file_link_count = 0
            for link_entry in json_data["links"]:
                source_file = link_entry.get("source_file")
                source_id = link_entry.get("source_id")
                target_file = link_entry.get("target_file")
                if not (source_file and source_id and target_file):
                    continue
                link_data.setdefault(source_file, {})[source_id] = {
                    "target_file": target_file,
                    "line": link_entry.get("source_line", 0),
                    "description": link_entry.get("description", ""),
                }
                file_link_count += 1
            logger.info(
                "Loaded %d links from %s",
                file_link_count,
                json_file.relative_to(search_dir),
            )
        except (json.JSONDecodeError, OSError) as exc:
            logger.warning("Failed to load %s: %s", json_file.name, exc)
    return link_data


def _collect_link_data(source_dir: Path) -> dict[str, dict[str, Any]]:
    """Load all ``*plantuml_links.json`` files from *source_dir*."""
    if source_dir.exists():
        return _load_link_mappings(str(source_dir))
    return {}


# ---------------------------------------------------------------------------
# UML injection helper
# ---------------------------------------------------------------------------


def _inject_links_into_uml(uml_content: str, links: dict[str, str]) -> str:
    """Append ``url of <alias> is [[url]]`` directives before ``@enduml``."""
    if not links:
        return uml_content
    safe_links = {
        alias: url
        for alias, url in links.items()
        if _ALIAS_SAFE_RE.match(alias) and "]]" not in url
    }
    if not safe_links:
        return uml_content
    url_directives = "\n".join(
        f"url of {alias} is [[{url}]]" for alias, url in safe_links.items()
    )
    enduml_match = re.search(r"^\s*@enduml\s*$", uml_content, re.MULTILINE)
    if enduml_match:
        prefix = uml_content[: enduml_match.start()]
        if not prefix.endswith("\n"):
            prefix += "\n"
        return prefix + url_directives + "\n" + uml_content[enduml_match.start() :]
    return uml_content + "\n" + url_directives


# ---------------------------------------------------------------------------
# Sphinx event handlers
# ---------------------------------------------------------------------------


@functools.lru_cache(maxsize=1)
def _get_plantuml_node_class() -> type | None:
    """Import the plantuml node class, returning ``None`` if unavailable."""
    try:
        from sphinxcontrib.plantuml import plantuml as PlantumlNode  # type: ignore[import-not-found]  # pylint: disable=import-outside-toplevel

        return PlantumlNode  # type: ignore[return-value]
    except ImportError:
        return None


def on_builder_inited(app: Sphinx) -> None:
    """Load JSON link data once, before any documents are read."""
    if app.builder.format != "html":
        return

    source_dir = Path(app.srcdir)
    link_data = _collect_link_data(source_dir)
    if not link_data:
        logger.info("clickable_plantuml: no link mappings found")
        return

    # Normalise keys to basenames for consistent lookup.
    normalized = {Path(k).name: v for k, v in link_data.items()}
    setattr(app.env, _ENV_LINK_DATA, normalized)

    logger.info(
        "clickable_plantuml: loaded links for %d source file(s)", len(normalized)
    )


def on_doctree_read(app: Sphinx, doctree: nodes.document) -> None:
    """Record which docname (and section anchor) contains which ``.puml`` diagram.

    Traverses the parsed doctree.
    The mapping is stored in ``app.env`` and consumed during ``doctree-resolved``.
    """
    PlantumlNode = _get_plantuml_node_class()
    if PlantumlNode is None:
        return

    puml_docnames: dict[str, tuple[str, str | None]] = getattr(
        app.env, _ENV_PUML_DOCNAMES, {}
    )

    for node in doctree.traverse(PlantumlNode):
        filename = Path(node.get("filename", "")).name
        if not filename:
            continue
        if filename in puml_docnames:
            logger.warning(
                "clickable_plantuml: diagram '%s' found in both '%s' and '%s' "
                "(basename collision — last wins)",
                filename,
                puml_docnames[filename][0],
                app.env.docname,
            )
        anchor = _find_parent_section_id(node)
        puml_docnames[filename] = (app.env.docname, anchor)

    setattr(app.env, _ENV_PUML_DOCNAMES, puml_docnames)


def on_doctree_resolved(app: Sphinx, doctree: nodes.document, docname: str) -> None:
    """Inject ``url of <alias> is [[url]]`` into plantuml nodes before rendering.

    For each diagram, resolves target ``.puml`` references to the docname that
    contains the target diagram and uses ``app.builder.get_relative_uri`` to
    produce correct relative URLs.
    """
    link_data: dict[str, dict[str, Any]] = getattr(app.env, _ENV_LINK_DATA, {})
    if app.builder.format != "html" or not link_data:
        return

    PlantumlNode = _get_plantuml_node_class()
    if PlantumlNode is None:
        return

    puml_docnames: dict[str, tuple[str, str | None]] = getattr(
        app.env, _ENV_PUML_DOCNAMES, {}
    )
    absolute_url_prefixes = ("http://", "https://", "/")

    modified_count = 0
    for node in doctree.traverse(PlantumlNode):
        diagram_filename = Path(node.get("filename", "")).name
        alias_map: dict[str, Any] = link_data.get(diagram_filename, {})
        if not alias_map:
            continue

        resolved_links: dict[str, str] = {}
        for alias, info in alias_map.items():
            target_file: str = info["target_file"]

            if target_file.endswith(".puml"):
                target_basename = Path(target_file).name
                target_info = puml_docnames.get(target_basename)
                if target_info is not None:
                    target_docname, target_anchor = target_info
                    # SVG files are stored in _images/ (one level below the
                    # HTML output root). Using get_relative_uri() would give a
                    # page-to-page relative URL, but that path is interpreted
                    # relative to the SVG file, not the parent HTML page —
                    # causing the browser to open the raw SVG. Instead, build
                    # the URL relative to _images/ by prepending "../" to the
                    # root-relative page URI returned by get_target_uri().
                    page_uri = app.builder.get_target_uri(target_docname)
                    url = f"../{page_uri}"
                    if target_anchor:
                        url += f"#{target_anchor}"
                    resolved_links[alias] = url
                else:
                    logger.debug(
                        "clickable_plantuml: target diagram '%s' for alias "
                        "'%s' not found in any document",
                        target_file,
                        alias,
                    )
            elif target_file.startswith(absolute_url_prefixes):
                resolved_links[alias] = target_file
            else:
                resolved_links[alias] = target_file

        if resolved_links:
            node["uml"] = _inject_links_into_uml(node.get("uml", ""), resolved_links)
            modified_count += 1

    if modified_count:
        logger.debug(
            "clickable_plantuml: injected links into %d diagram(s) on '%s'",
            modified_count,
            docname,
        )


def on_env_purge_doc(app: Sphinx, env: Any, docname: str) -> None:
    """Remove stale entries when a document is re-read (incremental builds)."""
    puml_docnames: dict[str, tuple[str, str | None]] = getattr(
        env, _ENV_PUML_DOCNAMES, {}
    )
    keys_to_remove = [k for k, (dn, _) in puml_docnames.items() if dn == docname]
    for k in keys_to_remove:
        del puml_docnames[k]


def on_env_merge_info(app: Sphinx, env: Any, docnames: set[str], other: Any) -> None:
    """Merge diagram location data from parallel read sub-processes."""
    puml_docnames: dict[str, tuple[str, str | None]] = getattr(
        env, _ENV_PUML_DOCNAMES, {}
    )
    other_map: dict[str, tuple[str, str | None]] = getattr(
        other, _ENV_PUML_DOCNAMES, {}
    )
    puml_docnames.update(other_map)
    setattr(env, _ENV_PUML_DOCNAMES, puml_docnames)


def setup(app: Sphinx) -> dict[str, Any]:
    """Register the extension with Sphinx."""
    app.connect("builder-inited", on_builder_inited)
    app.connect("doctree-read", on_doctree_read)
    app.connect("doctree-resolved", on_doctree_resolved)
    app.connect("env-purge-doc", on_env_purge_doc)
    app.connect("env-merge-info", on_env_merge_info)

    return {
        "version": "4.0",
        "parallel_read_safe": True,
        "parallel_write_safe": True,
    }
