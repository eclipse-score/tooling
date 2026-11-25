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

The links come from ``diagram_links.json`` files that ``rules_score`` stages
into the Sphinx source tree (``validation_cli --links-output``). The link rules
live in the validation tool (see ``validation/core/README.md``); this extension
only maps diagrams to documents and writes the links into the PlantUML source:

* an **element link** ``(source, alias, target)`` becomes
  ``url of <alias> is [[url]]`` in the diagram ``source``;
* a **call link** ``(source, line, method, targets)`` wraps the label of the
  message on ``line`` in ``[[url label]]``. The first target that is part of a
  document gets the link; with more than one, a tooltip names all of them.

A link whose target diagram is not part of any document is skipped.
"""

from __future__ import annotations

import functools
import hashlib
import json
import os
import re
import urllib.parse
from collections.abc import Callable
from pathlib import Path, PurePosixPath
from typing import Any

from docutils import nodes
from sphinx.application import Sphinx
from sphinx.util import logging

logger = logging.getLogger(__name__)

_LINKS_FILE = "diagram_links.json"
_LINKS_VERSION = 1

# ---------------------------------------------------------------------------
# Environment attribute names
# ---------------------------------------------------------------------------

# {source_key: {alias: target_key}} — loaded once in builder-inited.
_ENV_ELEMENT_LINKS = "clickable_plantuml_element_links"
# {source_key: [(line, method, [target_key, ...])]} — loaded once in builder-inited.
_ENV_CALL_LINKS = "clickable_plantuml_call_links"
# {normalized_source_path: (docname, anchor_or_None)} — populated in doctree-read.
_ENV_PUML_DOCNAMES = "clickable_plantuml_puml_docnames"
# Absolute prefix to strip from PlantUML node paths to obtain canonical source keys.
_ENV_WORKSPACE_OFFSET = "clickable_plantuml_workspace_offset"
# Frozen set of known canonical source keys for fast membership checks.
_ENV_SOURCE_KEYS = "clickable_plantuml_source_keys"
# {docname: count} of plantuml nodes that are in no link, per document.
_ENV_UNLINKED_NODE_COUNTS_BY_DOC = "clickable_plantuml_unlinked_node_counts_by_doc"

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
# Links loading
# ---------------------------------------------------------------------------

_CallLink = tuple[int, str, list[str]]


def _load_diagram_links(
    source_dir: Path,
) -> tuple[dict[str, dict[str, str]], dict[str, list[_CallLink]]]:
    """Read every ``diagram_links.json`` below *source_dir*.

    Files are read in sorted order; for a link given twice the first wins.

    Returns:
        element_links: ``{source_key → {alias → target_key}}``
        call_links:    ``{source_key → [(line, method, [target_keys])]}``
    """
    element_links: dict[str, dict[str, str]] = {}
    calls: dict[str, dict[tuple[int, str], list[str]]] = {}

    for json_path in sorted(source_dir.rglob(_LINKS_FILE)):
        try:
            data = json.loads(json_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError) as exc:
            logger.warning("clickable_plantuml: failed to load %s: %s", json_path, exc)
            continue
        if data.get("version") != _LINKS_VERSION:
            logger.warning(
                "clickable_plantuml: %s has version %r, expected %d — skipped",
                json_path,
                data.get("version"),
                _LINKS_VERSION,
            )
            continue

        for link in data.get("element_links", []):
            source = _normalize_source_path(link["source"])
            element_links.setdefault(source, {}).setdefault(link["alias"], _normalize_source_path(link["target"]))
        for link in data.get("call_links", []):
            source = _normalize_source_path(link["source"])
            targets = [_normalize_source_path(target) for target in link["targets"]]
            calls.setdefault(source, {}).setdefault((link["line"], link["method"]), targets)

    call_links = {
        source: [(line, method, targets) for (line, method), targets in sorted(by_call.items())]
        for source, by_call in calls.items()
    }
    logger.info(
        "clickable_plantuml: loaded links of %d diagram(s)",
        len(set(element_links) | set(call_links)),
    )
    return element_links, call_links


def _links_digest(source_dir: Path) -> str:
    """Digest of every ``diagram_links.json`` below *source_dir*."""
    digest = hashlib.sha256()
    for json_path in sorted(source_dir.rglob(_LINKS_FILE)):
        try:
            content = json_path.read_bytes()
        except OSError:
            continue
        digest.update(json_path.relative_to(source_dir).as_posix().encode())
        digest.update(content)
    return digest.hexdigest()


def _source_keys(element_links: dict[str, dict[str, str]], call_links: dict[str, list[_CallLink]]) -> frozenset[str]:
    """Return every diagram that takes part in a link, as source or target."""
    keys: set[str] = set(element_links) | set(call_links)
    for links in element_links.values():
        keys.update(links.values())
    for links in call_links.values():
        for _, _, targets in links:
            keys.update(targets)
    for key in keys:
        _assert_canonical_source_key(key)
    return frozenset(keys)


# ---------------------------------------------------------------------------
# UML injection
# ---------------------------------------------------------------------------

# Characters allowed in a bare (unquoted) PlantUML alias identifier.
_ALIAS_SAFE_RE = re.compile(r"^[\w.\-]+$")
# Any other code is written quoted; a quote or line break cannot be quoted.
_ALIAS_QUOTABLE_RE = re.compile(r'^[^"\r\n]+$')
# Matches the @enduml terminator line (used to inject url directives before it).
_ENDUML_RE = re.compile(r"^\s*@enduml\s*$", re.MULTILINE)


def _url_directive(alias: str, url: str) -> str | None:
    """Return the ``url of`` directive for *alias*, or ``None`` if it cannot be written."""
    if _ALIAS_SAFE_RE.match(alias):
        return f"url of {alias} is [[{url}]]"
    if _ALIAS_QUOTABLE_RE.match(alias):
        return f'url of "{alias}" is [[{url}]]'
    logger.warning("clickable_plantuml: alias %r cannot be written in a url directive — no link emitted", alias)
    return None


def _inject_links_into_uml(uml_content: str, links: dict[str, str]) -> str:
    """Append ``url of <alias> is [[url]]`` directives before ``@enduml``."""
    if not links:
        return uml_content

    directives = [d for alias, url in links.items() if (d := _url_directive(alias, url)) is not None]
    if not directives:
        return uml_content

    url_directives = "\n".join(directives)
    enduml_match = _ENDUML_RE.search(uml_content)
    if enduml_match:
        prefix = uml_content[: enduml_match.start()]
        if not prefix.endswith("\n"):
            prefix += "\n"
        return prefix + url_directives + "\n" + uml_content[enduml_match.start() :]
    return uml_content + "\n" + url_directives


# ---------------------------------------------------------------------------
# Call links
# ---------------------------------------------------------------------------


def _split_message_label(line: str) -> tuple[str, str] | None:
    """Split a sequence message line into ``(prefix, label)`` at the first colon outside quotes."""
    in_quotes = False
    for position, char in enumerate(line):
        if char == '"':
            in_quotes = not in_quotes
        elif char == ":" and not in_quotes:
            return line[: position + 1], line[position + 1 :].strip()
    return None


def _names_method(label: str, method: str) -> bool:
    """Return whether *label* is a call of *method*, with or without arguments."""
    rest = label.removeprefix(method)
    return rest != label and (not rest or rest.lstrip().startswith("("))


def _link_message_label(uml: str, line: int, method: str, url: str, tooltip: str | None) -> str:
    """Return *uml* with the label of the message on 1-based *line* wrapped in ``[[url label]]``.

    The line stays unchanged unless it is a message whose label names
    *method* and holds no link yet (guard against a stale links file).
    """
    lines = uml.split("\n")
    if not 1 <= line <= len(lines):
        return uml
    split = _split_message_label(lines[line - 1])
    if split is None:
        return uml
    prefix, label = split
    if "[[" in label or "]]" in label or not _names_method(label, method):
        return uml
    tooltip_text = f"{{{tooltip}}}" if tooltip else ""
    lines[line - 1] = f"{prefix} [[{url}{tooltip_text} {label}]]"
    return "\n".join(lines)


def _link_calls(
    uml: str,
    links: list[_CallLink],
    url_for: Callable[[str], str | None],
) -> str:
    """Link each call label of a sequence diagram to its detail diagrams.

    The first detail that is part of a document gets the link; when several
    diagrams refine the call the tooltip names all of them.
    """
    for line, method, targets in links:
        details = [(target, url) for target in targets if (url := url_for(target)) is not None]
        if not details:
            continue
        names = ", ".join(PurePosixPath(detail).stem for detail, _ in details)
        tooltip = f"Refined by: {names}" if len(details) > 1 and "}" not in names else None
        uml = _link_message_label(uml, line, method, _escape_plantuml_url(details[0][1]), tooltip)
    return uml


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


def _has_path_suffix(path_value: str, suffix_value: str) -> bool:
    """Return whether *suffix_value*'s path components are a tail of *path_value*'s."""
    path_parts = PurePosixPath(path_value).parts
    suffix_parts = PurePosixPath(suffix_value).parts
    if len(path_parts) < len(suffix_parts):
        return False
    return path_parts[-len(suffix_parts) :] == suffix_parts


def _unique_suffix_source_key(path_value: str, source_keys: set[str]) -> str | None:
    """Return the single canonical source key that is a path-suffix match, or ``None``.

    Used only as a safe fallback for symlinked/absolute staging layouts where
    the exact workspace-offset prefix match fails. A match is only trusted
    when it is *unique*: zero or multiple candidates return ``None`` rather
    than guessing (safe over wrong), so this can never introduce the
    basename-collision ambiguity the exact-match path is designed to avoid.
    """
    canonical_path = _normalize_source_path(path_value)
    if canonical_path in source_keys:
        return canonical_path
    matches = [
        key for key in source_keys if _has_path_suffix(canonical_path, key) or _has_path_suffix(key, canonical_path)
    ]
    if len(matches) == 1:
        return matches[0]
    return None


def _node_source_key(node: nodes.Node, srcdir: str, workspace_offset: str, source_keys: set[str]) -> str | None:
    """Return the canonical workspace-relative key for a plantuml *node*.

    ``sphinxcontrib.plantuml`` stores the diagram location on the node as
    ``incdir`` (directory relative to Sphinx's source root) plus ``filename``
    (bare basename).  Resolution has two tiers, both deterministic:

    1. **Exact match** – both the node path and ``workspace_offset`` are
       resolved via ``os.path.realpath`` and the workspace-offset prefix is
       stripped; the remainder must equal a canonical source key exactly.
    2. **Unique suffix match** – a safe fallback for symlinked/absolute
       staging layouts (see [`_unique_suffix_source_key`][]): only used when
       exactly one canonical source key shares a path suffix with the node's
       path, never a best-effort guess among several candidates.

    There is no ambiguous/basename-overlap fallback: when neither tier yields
    a unique answer, this returns ``None`` rather than guessing, so
    same-basename diagrams in different packages never silently mislink.

    Returns ``None`` when the node carries no filename, or no tier above
    resolves to a unique canonical key in ``source_keys``.
    """
    filename: str = node.get("filename", "")
    if not filename:
        return None
    srcdir = os.fspath(srcdir)
    incdir: str = node.get("incdir", "")
    node_abs = PurePosixPath(os.path.normpath(os.path.join(srcdir, incdir, filename))).as_posix()

    workspace_offset = PurePosixPath(os.path.realpath(os.fspath(workspace_offset))).as_posix()
    node_abs_real = PurePosixPath(os.path.realpath(node_abs)).as_posix()

    workspace_offset = workspace_offset.rstrip("/")
    if node_abs_real.startswith(workspace_offset + "/"):
        source_key = _normalize_source_path(node_abs_real[len(workspace_offset) + 1 :])
        if source_key in source_keys:
            _assert_canonical_source_key(source_key)
            return source_key

    candidate_paths = list(
        dict.fromkeys(
            (
                node_abs,
                node_abs_real,
                os.path.normpath(os.path.join(incdir, filename)),
            )
        )
    )
    for candidate_path in candidate_paths:
        source_key = _unique_suffix_source_key(candidate_path, source_keys)
        if source_key is not None:
            _assert_canonical_source_key(source_key)
            return source_key

    return None


# ---------------------------------------------------------------------------
# Sphinx event handlers
# ---------------------------------------------------------------------------


def on_config_inited(app: Sphinx, config: Any) -> None:
    """Make a change of the links a config change: Sphinx then re-reads all documents."""
    source_dir = Path(app.srcdir)
    if source_dir.exists():
        config.clickable_plantuml_links_digest = _links_digest(source_dir)


def on_builder_inited(app: Sphinx) -> None:
    """Load the diagram links once."""
    if app.builder.format != "html":
        return

    source_dir = Path(app.srcdir)
    if not source_dir.exists():
        logger.info("clickable_plantuml: srcdir does not exist — no links loaded")
        return

    element_links, call_links = _load_diagram_links(source_dir)
    source_keys = _source_keys(element_links, call_links)
    if not source_keys:
        logger.info("clickable_plantuml: no %s found — diagrams stay without links", _LINKS_FILE)
        return

    setattr(app.env, _ENV_WORKSPACE_OFFSET, _compute_workspace_offset(app.srcdir, set(source_keys)))
    setattr(app.env, _ENV_ELEMENT_LINKS, element_links)
    setattr(app.env, _ENV_CALL_LINKS, call_links)
    setattr(app.env, _ENV_SOURCE_KEYS, source_keys)
    setattr(app.env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {})


def on_doctree_read(app: Sphinx, doctree: nodes.document) -> None:
    """Record which docname (and section anchor) contains which diagram.

    Each diagram is registered under its canonical workspace-relative key (a
    ``source``/``target`` of ``diagram_links.json`` matched to the node's
    absolute path).
    """
    PlantumlNode = _get_plantuml_node_class()
    if PlantumlNode is None:
        return

    if app.builder.format != "html":
        return

    source_keys: frozenset[str] = getattr(app.env, _ENV_SOURCE_KEYS, frozenset())
    if not source_keys:
        return

    workspace_offset: str = getattr(app.env, _ENV_WORKSPACE_OFFSET, app.srcdir)
    puml_docnames: dict[str, tuple[str, str | None]] = getattr(app.env, _ENV_PUML_DOCNAMES, {})
    unlinked_counts_by_doc: dict[str, int] = getattr(app.env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {})

    # Recomputed per (re-)read of this docname so a rebuild does not double-count.
    doc_unlinked_count = 0

    for node in doctree.findall(PlantumlNode):
        key = _node_source_key(node, app.srcdir, workspace_offset, source_keys)
        if not key:
            logger.debug(
                "clickable_plantuml: plantuml node in '%s' has no resolvable source path — skipped",
                app.env.docname,
            )
            doc_unlinked_count += 1
            continue
        if key in puml_docnames and puml_docnames[key][0] != app.env.docname:
            logger.warning(
                "clickable_plantuml: diagram '%s' found in both '%s' and '%s' — last wins (path collision)",
                key,
                puml_docnames[key][0],
                app.env.docname,
            )
        anchor = _find_parent_section_id(node)
        puml_docnames[key] = (app.env.docname, anchor)

    if doc_unlinked_count:
        unlinked_counts_by_doc[app.env.docname] = doc_unlinked_count
    else:
        unlinked_counts_by_doc.pop(app.env.docname, None)

    setattr(app.env, _ENV_PUML_DOCNAMES, puml_docnames)
    setattr(app.env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, unlinked_counts_by_doc)


def on_build_finished(app: Sphinx, exception: Exception | None) -> None:
    """Emit an aggregate info log for plantuml nodes that are in no link."""
    if app.builder.format != "html" or exception is not None:
        return

    unlinked_counts_by_doc: dict[str, int] = getattr(app.env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {})
    total = sum(unlinked_counts_by_doc.values())
    if total:
        logger.info(
            "clickable_plantuml: %d plantuml nodes are in no diagram link",
            total,
        )


def on_doctree_resolved(app: Sphinx, doctree: nodes.document, docname: str) -> None:
    """Write the element and call links into the plantuml nodes.

    The URL base depends on the configured ``plantuml_output_format``:

    * ``svg_obj`` – the rendered SVG lives in the ``_images/`` directory and is
      embedded via ``<object>``; ``<a href>`` targets inside the SVG resolve
      relative to ``_images/``, so the URL is
      ``os.path.relpath(target_uri, imagedir)``.
    * inline ``svg`` / ``png`` – the link resolves relative to the containing
      HTML page, so the URL is
      ``app.builder.get_relative_uri(docname, target_docname)``.
    """
    source_keys: frozenset[str] = getattr(app.env, _ENV_SOURCE_KEYS, frozenset())
    if app.builder.format != "html" or not source_keys:
        return

    PlantumlNode = _get_plantuml_node_class()
    if PlantumlNode is None:
        return

    element_links: dict[str, dict[str, str]] = getattr(app.env, _ENV_ELEMENT_LINKS, {})
    call_links: dict[str, list[_CallLink]] = getattr(app.env, _ENV_CALL_LINKS, {})
    workspace_offset: str = getattr(app.env, _ENV_WORKSPACE_OFFSET, app.srcdir)
    puml_docnames: dict[str, tuple[str, str | None]] = getattr(app.env, _ENV_PUML_DOCNAMES, {})
    output_format = getattr(app.config, "plantuml_output_format", "png")
    imagedir = getattr(app.builder, "imagedir", "_images")

    def _url_for(target: str) -> str | None:
        target_info = puml_docnames.get(target)
        if target_info is None:
            logger.debug("clickable_plantuml: diagram '%s' is in no document — no link", target)
            return None
        return _build_target_url(app.builder, output_format, imagedir, docname, *target_info)

    modified_count = 0
    for node in doctree.findall(PlantumlNode):
        source_key = _node_source_key(node, app.srcdir, workspace_offset, source_keys)
        if not source_key:
            continue

        uml = node.get("uml", "")
        resolved_links = {
            alias: _escape_plantuml_url(url)
            for alias, target in element_links.get(source_key, {}).items()
            if (url := _url_for(target)) is not None
        }
        linked_uml = _inject_links_into_uml(uml, resolved_links)
        linked_uml = _link_calls(linked_uml, call_links.get(source_key, []), _url_for)
        if linked_uml != uml:
            node["uml"] = linked_uml
            modified_count += 1

    if modified_count:
        logger.debug(
            "clickable_plantuml: injected links into %d diagram(s) on '%s'",
            modified_count,
            docname,
        )


def on_env_purge_doc(app: Sphinx, env: Any, docname: str) -> None:
    """Remove stale entries when a document is re-read (incremental builds)."""
    puml_docnames: dict[str, tuple[str, str | None]] = getattr(env, _ENV_PUML_DOCNAMES, {})
    keys_to_remove = [k for k, (dn, _) in puml_docnames.items() if dn == docname]
    for k in keys_to_remove:
        del puml_docnames[k]

    unlinked_counts_by_doc: dict[str, int] = getattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {})
    unlinked_counts_by_doc.pop(docname, None)


def on_env_merge_info(app: Sphinx, env: Any, docnames: set[str], other: Any) -> None:
    """Merge diagram location data from parallel read sub-processes."""
    puml_docnames: dict[str, tuple[str, str | None]] = getattr(env, _ENV_PUML_DOCNAMES, {})
    other_map: dict[str, tuple[str, str | None]] = getattr(other, _ENV_PUML_DOCNAMES, {})
    puml_docnames.update(other_map)
    setattr(env, _ENV_PUML_DOCNAMES, puml_docnames)

    unlinked_counts_by_doc: dict[str, int] = getattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {})
    other_unlinked_counts_by_doc: dict[str, int] = getattr(other, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {})
    unlinked_counts_by_doc.update(other_unlinked_counts_by_doc)
    setattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, unlinked_counts_by_doc)


def setup(app: Sphinx) -> dict[str, Any]:
    """Register the extension with Sphinx."""
    app.add_config_value("clickable_plantuml_links_digest", "", "env")
    app.connect("config-inited", on_config_inited)
    app.connect("builder-inited", on_builder_inited)
    app.connect("doctree-read", on_doctree_read)
    app.connect("doctree-resolved", on_doctree_resolved)
    app.connect("env-purge-doc", on_env_purge_doc)
    app.connect("env-merge-info", on_env_merge_info)
    app.connect("build-finished", on_build_finished)

    return {
        "version": "6.0",
        "parallel_read_safe": True,
        "parallel_write_safe": True,
    }
