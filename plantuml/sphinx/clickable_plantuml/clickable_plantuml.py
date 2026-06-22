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
"""Sphinx extension to make PlantUML diagrams clickable.

Design overview
---------------
Link data is derived from ``.idmap.json`` sidecar files produced by the
PlantUML parser (``puml_cli --idmap-output-dir ...``).  Each idmap file
records two roles for the elements in one ``.puml`` diagram:

* **defines** – elements elaborated (given children / structure) in that
  diagram.  A component diagram that contains ``package Proxy { ... }`` is
  the definition site of ``Proxy``.
* **references** – leaf mentions and relation endpoints.  A top-level
  ``[Proxy]`` box in an overview is a reference that should link to the
  diagram that defines it.

The matching algorithm:

1. Build a *definition index*: ``{alias|id → [source_paths]}``.
2. For each reference ``(alias, id)`` in a diagram, look up the index (FQN
   ``id`` first, then ``alias``) to find candidate definer diagrams.
3. If exactly one definer: emit the link.
4. If multiple definers: pick the one sharing the longest common workspace-
   relative path prefix with the source diagram (proximity tiebreak).
   On a tie: log a warning and emit no link (safe over wrong).
5. Never link a diagram to itself.
"""

from __future__ import annotations

import functools
import json
import os
import re
import urllib.parse
from pathlib import Path, PurePosixPath
from typing import Any

from docutils import nodes
from sphinx.application import Sphinx
from sphinx.errors import ExtensionError
from sphinx.util import logging

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Environment attribute names
# ---------------------------------------------------------------------------

# {normalized_source_path: raw_idmap_dict} — loaded once in builder-inited.
_ENV_IDMAP_BY_SOURCE = "clickable_plantuml_idmap_by_source"
# {alias_or_id: [source_path, ...]} — definition index built in builder-inited.
_ENV_DEF_INDEX = "clickable_plantuml_def_index"
# {normalized_source_path: (docname, anchor_or_None)} — populated in doctree-read.
_ENV_PUML_DOCNAMES = "clickable_plantuml_puml_docnames"
# Absolute prefix to strip from PlantUML node paths to obtain canonical source keys.
_ENV_WORKSPACE_OFFSET = "clickable_plantuml_workspace_offset"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


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


def _normalize_source_path(raw: str) -> str:
    """Normalise a source path to a forward-slash relative string."""
    return str(PurePosixPath(raw)).lstrip("/")


def _assert_canonical_source_key(source_key: str) -> None:
    """Assert that *source_key* is a workspace-relative POSIX key."""
    if source_key != _normalize_source_path(source_key):
        raise ValueError(f"non-canonical source key: {source_key!r}")


def _common_prefix_length(path_a: str, path_b: str) -> int:
    """Return the number of shared path components between two canonical keys."""
    parts_a = PurePosixPath(path_a).parts
    parts_b = PurePosixPath(path_b).parts
    count = 0
    for a, b in zip(parts_a, parts_b):
        if a == b:
            count += 1
        else:
            break
    return count


def _proximity_tiebreak(source: str, candidates: list[str]) -> str | None:
    """Pick the candidate with the longest common prefix with *source*.

    All inputs are canonical workspace-relative POSIX keys (guaranteed by the
    exact-matching in P0-1); the assertions guard that invariant so a staging
    path can never sneak into the comparison.  Returns ``None`` when two or
    more candidates score equally (tie → no link).
    """
    _assert_canonical_source_key(source)
    for candidate in candidates:
        _assert_canonical_source_key(candidate)
    scored = sorted(
        candidates,
        key=lambda c: _common_prefix_length(source, c),
        reverse=True,
    )
    best = _common_prefix_length(source, scored[0])
    if len(scored) > 1 and _common_prefix_length(source, scored[1]) == best:
        return None
    return scored[0]


def _resolve_definer(
    alias: str,
    fqn: str,
    source_key: str,
    definition_index: dict[str, list[str]],
) -> str | None:
    """Return the definer source key for one reference, or ``None``.

    Resolution rules:

    * FQN (``id``) lookup takes precedence over the ``alias`` lookup.
    * A diagram never links to itself (self-links are dropped).
    * A single remaining candidate wins outright; multiple candidates go
      through the proximity tiebreak, and a genuine tie logs a warning and
      returns ``None`` (safe over wrong).
    """
    _assert_canonical_source_key(source_key)
    candidates = definition_index.get(fqn) or definition_index.get(alias) or []
    for candidate in candidates:
        _assert_canonical_source_key(candidate)
    # Never link a diagram to itself.
    candidates = [c for c in candidates if c != source_key]
    if not candidates:
        return None
    if len(candidates) == 1:
        return candidates[0]
    target = _proximity_tiebreak(source_key, candidates)
    if target is None:
        logger.warning(
            "clickable_plantuml: ambiguous definition for '%s' in '%s'"
            " — tied candidates %s; no link emitted",
            alias,
            source_key,
            candidates,
        )
    return target


def _build_target_url(
    builder: Any,
    output_format: str,
    imagedir: str,
    docname: str,
    target_docname: str,
    anchor: str | None,
) -> str:
    """Build the link URL for a resolved definer diagram.

    In ``svg_obj`` mode the rendered SVG lives in the ``_images/`` directory,
    so URLs inside the SVG must be relative to ``_images/`` rather than the
    containing HTML page.  For inline ``svg``/``png`` the SVG is embedded in
    the page, so a page-relative URL is correct.  The optional section
    *anchor* is appended as a fragment.
    """
    if output_format == "svg_obj":
        target_uri = builder.get_target_uri(target_docname)
        url = os.path.relpath(target_uri, imagedir).replace("\\", "/")
    else:
        url = builder.get_relative_uri(docname, target_docname)
    if anchor:
        url += f"#{anchor}"
    return url


def _escape_plantuml_url(url: str) -> str:
    """Percent-encode characters significant in PlantUML URL syntax.

    PlantUML terminates ``url of X is [[...]]`` at the first ``]]``.  We also
    encode ``[``, spaces, and other characters that would confuse the PlantUML
    lexer.  The fragment (after ``#``) is encoded separately to preserve it.
    """
    # Characters that are safe to leave unencoded in a URL context. ``#`` is
    # deliberately excluded here and reintroduced only as the single fragment
    # separator, so literal hash payload is always encoded.
    _SAFE = "/:?&=@!$'()*+,;-._~"
    fragment_sep = url.find("#")
    if fragment_sep != -1:
        base = urllib.parse.quote(url[:fragment_sep], safe=_SAFE)
        frag = urllib.parse.quote(url[fragment_sep + 1 :], safe="-._~")
        # Keep a real fragment separator so generated SVG href remains valid.
        return f"{base}#{frag}"
    return urllib.parse.quote(url, safe=_SAFE)


# ---------------------------------------------------------------------------
# idmap loading
# ---------------------------------------------------------------------------


def _load_idmap_files(
    source_dir: Path,
) -> tuple[dict[str, Any], dict[str, list[str]]]:
    """Scan *source_dir* for ``*.idmap.json`` and build the lookup indices.

    The canonical key is the workspace-relative POSIX path stored in each
    idmap's ``source`` field (baked in by ``--source-name``).  Matching is
    exact — there is no basename fallback — so two same-basename diagrams in
    different packages never mislink.

    Returns:
        idmap_by_source:   ``{canonical_source_key → raw idmap dict}``
        definition_index:  ``{alias_or_fqn_id → [canonical_source_keys]}``

    Raises:
        ExtensionError: when two idmaps normalise to the same canonical key.
    """
    idmap_by_source: dict[str, Any] = {}
    definition_index: dict[str, list[str]] = {}

    for json_path in sorted(source_dir.rglob("*.idmap.json")):
        try:
            data = json.loads(json_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError) as exc:
            logger.warning("clickable_plantuml: failed to load %s: %s", json_path, exc)
            continue

        raw_source = data.get("source", "")
        if not raw_source:
            logger.warning(
                "clickable_plantuml: idmap %s missing 'source' field — skipped",
                json_path.name,
            )
            continue

        source_key = _normalize_source_path(raw_source)
        _assert_canonical_source_key(source_key)
        if source_key in idmap_by_source:
            raise ExtensionError(
                "clickable_plantuml: duplicate idmap source key "
                f"'{source_key}' (from {json_path.name}); each diagram's "
                "--source-name must be a unique workspace-relative path."
            )
        idmap_by_source[source_key] = data

        for entry in data.get("defines", []):
            alias = entry.get("alias", "")
            fqn = entry.get("id", "")
            if alias:
                definition_index.setdefault(alias, []).append(source_key)
            if fqn and fqn != alias:
                definition_index.setdefault(fqn, []).append(source_key)

    logger.info(
        "clickable_plantuml: loaded %d idmap file(s), %d unique definition keys",
        len(idmap_by_source),
        len(definition_index),
    )
    return idmap_by_source, definition_index


# ---------------------------------------------------------------------------
# UML injection
# ---------------------------------------------------------------------------

# Characters allowed in a PlantUML alias identifier.
_ALIAS_SAFE_RE = re.compile(r"^[\w.\-]+$")
# Matches the @enduml terminator line (used to inject url directives before it).
_ENDUML_RE = re.compile(r"^\s*@enduml\s*$", re.MULTILINE)


def _inject_links_into_uml(uml_content: str, links: dict[str, str]) -> str:
    """Append ``url of <alias> is [[url]]`` directives before ``@enduml``."""
    if not links:
        return uml_content
    safe_links = {
        alias: url for alias, url in links.items() if _ALIAS_SAFE_RE.match(alias)
    }
    if not safe_links:
        return uml_content
    url_directives = "\n".join(
        f"url of {alias} is [[{url}]]" for alias, url in safe_links.items()
    )
    enduml_match = _ENDUML_RE.search(uml_content)
    if enduml_match:
        prefix = uml_content[: enduml_match.start()]
        if not prefix.endswith("\n"):
            prefix += "\n"
        return prefix + url_directives + "\n" + uml_content[enduml_match.start() :]
    return uml_content + "\n" + url_directives


# ---------------------------------------------------------------------------
# plantuml node class (cached import)
# ---------------------------------------------------------------------------


@functools.lru_cache(maxsize=1)
def _get_plantuml_node_class() -> type | None:
    """Import the plantuml node class, returning ``None`` if unavailable."""
    try:
        from sphinxcontrib.plantuml import plantuml as PlantumlNode  # type: ignore[import-not-found]  # pylint: disable=import-outside-toplevel

        return PlantumlNode  # type: ignore[return-value]
    except ImportError:
        return None


# ---------------------------------------------------------------------------
# Node filename normalisation
# ---------------------------------------------------------------------------


def _compute_workspace_offset(srcdir: str, source_keys: set[str]) -> str:
    """Compute the absolute prefix used to derive canonical source keys.

    This runs once during ``builder-inited``. The returned prefix is removed
    from absolute PlantUML node paths to obtain exact workspace-relative keys.
    """
    srcdir_posix = PurePosixPath(os.path.normpath(srcdir)).as_posix()
    best_parent = ""

    for key in source_keys:
        _assert_canonical_source_key(key)
        parent = str(PurePosixPath(key).parent)
        if parent in ("", "."):
            continue
        if srcdir_posix == parent or srcdir_posix.endswith("/" + parent):
            if len(parent) > len(best_parent):
                best_parent = parent

    if not best_parent:
        return srcdir_posix
    if srcdir_posix == best_parent:
        return srcdir_posix
    return srcdir_posix[: -(len(best_parent) + 1)]


def _node_source_key(
    node: nodes.Node, srcdir: str, workspace_offset: str, source_keys: set[str]
) -> str | None:
    """Return the canonical workspace-relative key for a plantuml *node*.

    ``sphinxcontrib.plantuml`` stores the diagram location on the node as
    ``incdir`` (directory relative to Sphinx's source root) plus ``filename``
    (bare basename).  We first try strict prefix stripping via
    ``workspace_offset``.  If Bazel staging causes that to fail, we fall back
    to exact full-key suffix matching against canonical ``source_keys``.

    This remains collision-safe: we match full canonical keys only, never a
    basename-only key.

    Returns ``None`` when the node carries no filename or matches no key.
    """
    filename: str = node.get("filename", "")
    if not filename:
        return None
    incdir: str = node.get("incdir", "")
    node_abs = PurePosixPath(
        os.path.normpath(os.path.join(srcdir, incdir, filename))
    ).as_posix()

    workspace_offset = workspace_offset.rstrip("/")
    if node_abs.startswith(workspace_offset + "/"):
        source_key = _normalize_source_path(node_abs[len(workspace_offset) + 1 :])
        if source_key in source_keys:
            _assert_canonical_source_key(source_key)
            return source_key

    # Bazel staging can relocate docs while preserving the tail workspace path.
    matches = [
        key for key in source_keys if node_abs == key or node_abs.endswith("/" + key)
    ]
    if not matches:
        return None
    source_key = max(matches, key=len)
    _assert_canonical_source_key(source_key)
    return source_key


# ---------------------------------------------------------------------------
# Sphinx event handlers
# ---------------------------------------------------------------------------


def on_builder_inited(app: Sphinx) -> None:
    """Load idmap files and build the definition index once."""
    if app.builder.format != "html":
        return

    source_dir = Path(app.srcdir)
    if not source_dir.exists():
        logger.info("clickable_plantuml: srcdir does not exist — no idmaps loaded")
        return

    idmap_by_source, definition_index = _load_idmap_files(source_dir)
    if not idmap_by_source:
        logger.info("clickable_plantuml: no *.idmap.json files found")
        return

    workspace_offset = _compute_workspace_offset(app.srcdir, set(idmap_by_source))
    setattr(app.env, _ENV_WORKSPACE_OFFSET, workspace_offset)
    setattr(app.env, _ENV_IDMAP_BY_SOURCE, idmap_by_source)
    setattr(app.env, _ENV_DEF_INDEX, definition_index)


def on_doctree_read(app: Sphinx, doctree: nodes.document) -> None:
    """Record which docname (and section anchor) contains which diagram.

    Each diagram is registered under its canonical workspace-relative key (the
    idmap ``source`` matched to the node's absolute path), which is directly
    comparable to the idmap ``source`` field.
    """
    PlantumlNode = _get_plantuml_node_class()
    if PlantumlNode is None:
        return

    idmap_by_source: dict[str, Any] = getattr(app.env, _ENV_IDMAP_BY_SOURCE, {})
    source_keys = set(idmap_by_source)
    workspace_offset: str = getattr(app.env, _ENV_WORKSPACE_OFFSET, app.srcdir)
    puml_docnames: dict[str, tuple[str, str | None]] = getattr(
        app.env, _ENV_PUML_DOCNAMES, {}
    )

    for node in doctree.findall(PlantumlNode):
        key = _node_source_key(node, app.srcdir, workspace_offset, source_keys)
        if not key:
            logger.warning(
                "clickable_plantuml: plantuml node in '%s' has no resolvable"
                " source path — skipped",
                app.env.docname,
            )
            continue
        if key in puml_docnames and puml_docnames[key][0] != app.env.docname:
            logger.warning(
                "clickable_plantuml: diagram '%s' found in both '%s' and '%s'"
                " — last wins (path collision; check idmap source fields)",
                key,
                puml_docnames[key][0],
                app.env.docname,
            )
        anchor = _find_parent_section_id(node)
        puml_docnames[key] = (app.env.docname, anchor)

    setattr(app.env, _ENV_PUML_DOCNAMES, puml_docnames)


def on_doctree_resolved(app: Sphinx, doctree: nodes.document, docname: str) -> None:
    """Inject ``url of <alias> is [[url]]`` into plantuml nodes.

    Resolves each reference in the diagram's idmap to its definer diagram,
    applies a proximity tiebreak on ambiguity, and builds a URL whose base
    depends on the configured ``plantuml_output_format``:

    * ``svg_obj`` – the rendered SVG lives in the ``_images/`` directory and is
      embedded via ``<object>``; ``<a href>`` targets inside the SVG resolve
      relative to ``_images/``, so the URL is
      ``os.path.relpath(target_uri, imagedir)``.
    * inline ``svg`` / ``png`` – the link resolves relative to the containing
      HTML page, so the URL is
      ``app.builder.get_relative_uri(docname, target_docname)``.
    """
    idmap_by_source: dict[str, Any] = getattr(app.env, _ENV_IDMAP_BY_SOURCE, {})
    definition_index: dict[str, list[str]] = getattr(app.env, _ENV_DEF_INDEX, {})
    if app.builder.format != "html" or not idmap_by_source:
        return

    PlantumlNode = _get_plantuml_node_class()
    if PlantumlNode is None:
        return

    source_keys = set(idmap_by_source)
    workspace_offset: str = getattr(app.env, _ENV_WORKSPACE_OFFSET, app.srcdir)
    puml_docnames: dict[str, tuple[str, str | None]] = getattr(
        app.env, _ENV_PUML_DOCNAMES, {}
    )

    # Loop-invariant for the whole build: resolve once instead of per reference.
    output_format = getattr(app.config, "plantuml_output_format", "png")
    imagedir = getattr(app.builder, "imagedir", "_images")

    modified_count = 0
    for node in doctree.findall(PlantumlNode):
        source_key = _node_source_key(node, app.srcdir, workspace_offset, source_keys)
        if not source_key:
            continue

        idmap = idmap_by_source.get(source_key)
        if idmap is None:
            continue

        resolved_links: dict[str, str] = {}
        seen_aliases_in_node: set[str] = set()
        for ref in idmap.get("references", []):
            alias: str = ref.get("alias", "")
            fqn: str = ref.get("id", alias)
            if not alias or alias in seen_aliases_in_node:
                continue

            target_source = _resolve_definer(alias, fqn, source_key, definition_index)
            if target_source is None:
                continue

            target_info = puml_docnames.get(target_source)
            if target_info is None:
                logger.debug(
                    "clickable_plantuml: definer '%s' for alias '%s' not"
                    " found in any document — skipping",
                    target_source,
                    alias,
                )
                continue

            target_docname, target_anchor = target_info
            url = _build_target_url(
                app.builder,
                output_format,
                imagedir,
                docname,
                target_docname,
                target_anchor,
            )

            resolved_links[alias] = _escape_plantuml_url(url)
            seen_aliases_in_node.add(alias)

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
        "version": "5.0",
        "parallel_read_safe": True,
        "parallel_write_safe": True,
    }
