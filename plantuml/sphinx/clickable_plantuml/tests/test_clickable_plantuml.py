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
"""Unit tests for the clickable_plantuml Sphinx extension helpers."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from sphinx.errors import ExtensionError

from clickable_plantuml import (
    _build_target_url,
    _escape_plantuml_url,
    _inject_links_into_uml,
    _load_idmap_files,
    _node_source_key,
    _proximity_tiebreak,
    _resolve_definer,
)


def _write_idmap(
    directory: Path,
    name: str,
    source: str,
    defines: list[dict[str, str]] | None = None,
    references: list[dict[str, str]] | None = None,
) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    (directory / name).write_text(
        json.dumps(
            {
                "source": source,
                "defines": defines or [],
                "references": references or [],
            }
        ),
        encoding="utf-8",
    )


# ---------------------------------------------------------------------------
# index build
# ---------------------------------------------------------------------------


def test_load_idmap_builds_source_and_definition_indices(tmp_path: Path) -> None:
    _write_idmap(
        tmp_path / "a",
        "proxy.idmap.json",
        "pkg/a/proxy.puml",
        defines=[{"alias": "Proxy", "id": "pkg.Proxy"}],
    )
    _write_idmap(
        tmp_path / "b",
        "overview.idmap.json",
        "pkg/b/overview.puml",
        references=[{"alias": "Proxy", "id": "pkg.Proxy"}],
    )

    idmap_by_source, definition_index = _load_idmap_files(tmp_path)

    assert set(idmap_by_source) == {"pkg/a/proxy.puml", "pkg/b/overview.puml"}
    # Both the alias and the FQN point at the definer.
    assert definition_index["Proxy"] == ["pkg/a/proxy.puml"]
    assert definition_index["pkg.Proxy"] == ["pkg/a/proxy.puml"]


def test_same_basename_in_different_dirs_are_distinct_keys(tmp_path: Path) -> None:
    _write_idmap(tmp_path / "a", "proxy.idmap.json", "pkg/a/proxy.puml")
    _write_idmap(tmp_path / "b", "proxy.idmap.json", "pkg/b/proxy.puml")

    idmap_by_source, _ = _load_idmap_files(tmp_path)

    # No basename collapse: two proxy.puml remain independently keyed.
    assert set(idmap_by_source) == {"pkg/a/proxy.puml", "pkg/b/proxy.puml"}


def test_duplicate_canonical_key_raises_build_error(tmp_path: Path) -> None:
    _write_idmap(tmp_path / "a", "one.idmap.json", "pkg/dup.puml")
    _write_idmap(tmp_path / "b", "two.idmap.json", "pkg/dup.puml")

    with pytest.raises(ExtensionError, match="duplicate idmap source key"):
        _load_idmap_files(tmp_path)


# ---------------------------------------------------------------------------
# node source-key resolution (workspace-offset + exact key match)
# ---------------------------------------------------------------------------


def test_node_source_key_matches_workspace_relative_key() -> None:
    # sphinxcontrib.plantuml stores incdir + filename. After stripping the
    # workspace offset, the remainder must match a canonical idmap key exactly.
    node = {
        "filename": "overview.puml",
        "incdir": "plantuml/sphinx/example",
    }
    key = _node_source_key(
        node,
        "/workspace/plantuml/sphinx/example",
        "/workspace",
        {"plantuml/sphinx/example/overview.puml", "plantuml/sphinx/example/other.puml"},
    )

    assert key == "plantuml/sphinx/example/overview.puml"


def test_node_source_key_same_basename_stays_distinct() -> None:
    node = {"filename": "proxy.puml", "incdir": "pkg/b"}
    key = _node_source_key(
        node,
        "/workspace/pkg/b",
        "/workspace",
        {"pkg/a/proxy.puml", "pkg/b/proxy.puml"},
    )

    assert key == "pkg/b/proxy.puml"


def test_node_source_key_returns_none_when_unmatched() -> None:
    node = {"filename": "stray.puml", "incdir": "pkg/b"}
    key = _node_source_key(node, "/workspace/pkg/b", "/workspace", {"pkg/a/proxy.puml"})

    assert key is None


def test_node_source_key_returns_none_without_filename() -> None:
    assert (
        _node_source_key(
            {"incdir": "x"}, "/workspace/pkg", "/workspace", {"pkg/a.puml"}
        )
        is None
    )


def test_node_source_key_matches_bazel_staged_suffix_path() -> None:
    node = {
        "filename": "overview.puml",
        "incdir": "../../../src/plantuml/sphinx/example",
    }
    key = _node_source_key(
        node,
        "/build/out/doc/plantuml/sphinx/example",
        "/build/out/doc/plantuml/sphinx/example",
        {"plantuml/sphinx/example/overview.puml", "plantuml/sphinx/example/other.puml"},
    )

    assert key == "plantuml/sphinx/example/overview.puml"


# ---------------------------------------------------------------------------
# reference resolution: FQN-before-alias, self-link, single vs tie
# ---------------------------------------------------------------------------


def test_resolve_definer_prefers_fqn_over_alias() -> None:
    definition_index = {
        "Proxy": ["pkg/alias_hit.puml"],
        "pkg.Proxy": ["pkg/fqn_hit.puml"],
    }

    target = _resolve_definer("Proxy", "pkg.Proxy", "pkg/src.puml", definition_index)

    assert target == "pkg/fqn_hit.puml"


def test_resolve_definer_skips_self_link() -> None:
    definition_index = {"Proxy": ["pkg/src.puml"]}

    target = _resolve_definer("Proxy", "Proxy", "pkg/src.puml", definition_index)

    assert target is None


def test_resolve_definer_single_candidate() -> None:
    definition_index = {"Proxy": ["pkg/definer.puml"]}

    target = _resolve_definer("Proxy", "Proxy", "pkg/src.puml", definition_index)

    assert target == "pkg/definer.puml"


def test_resolve_definer_tie_returns_none() -> None:
    definition_index = {"Proxy": ["other/a/proxy.puml", "other/b/proxy.puml"]}

    target = _resolve_definer("Proxy", "Proxy", "pkg/src.puml", definition_index)

    assert target is None


def test_resolve_definer_proximity_breaks_tie() -> None:
    definition_index = {"Proxy": ["pkg/near/proxy.puml", "far/proxy.puml"]}

    target = _resolve_definer("Proxy", "Proxy", "pkg/src.puml", definition_index)

    assert target == "pkg/near/proxy.puml"


def test_resolve_definer_rejects_non_canonical_candidate() -> None:
    definition_index = {"Proxy": ["/abs/definer.puml"]}

    with pytest.raises(ValueError):
        _resolve_definer("Proxy", "Proxy", "pkg/src.puml", definition_index)


def test_resolve_definer_rejects_non_canonical_source_key() -> None:
    definition_index = {"Proxy": ["pkg/definer.puml"]}

    with pytest.raises(ValueError):
        _resolve_definer("Proxy", "Proxy", "/abs/src.puml", definition_index)


# ---------------------------------------------------------------------------
# proximity tiebreak
# ---------------------------------------------------------------------------


def test_proximity_tiebreak_single_winner() -> None:
    assert _proximity_tiebreak("a/b/c.puml", ["a/b/x.puml", "z/y.puml"]) == "a/b/x.puml"


def test_proximity_tiebreak_tie_returns_none() -> None:
    assert _proximity_tiebreak("a/b/c.puml", ["x/one.puml", "y/two.puml"]) is None


def test_proximity_tiebreak_rejects_non_canonical_key() -> None:
    with pytest.raises(ValueError):
        _proximity_tiebreak("/abs/src.puml", ["a/b.puml"])


# ---------------------------------------------------------------------------
# URL building: svg_obj vs svg, anchor
# ---------------------------------------------------------------------------


class _FakeBuilder:
    def get_target_uri(self, docname: str) -> str:
        return f"{docname}.html"

    def get_relative_uri(self, from_docname: str, to_docname: str) -> str:
        return f"{to_docname}.html"


def test_build_target_url_svg_obj_is_relative_to_imagedir() -> None:
    url = _build_target_url(
        _FakeBuilder(), "svg_obj", "_images", "index", "design/proxy", None
    )

    # svg_obj links resolve relative to _images/, so climb out of it first.
    assert url == "../design/proxy.html"


def test_build_target_url_inline_svg_is_page_relative() -> None:
    url = _build_target_url(
        _FakeBuilder(), "svg", "_images", "index", "design/proxy", None
    )

    assert url == "design/proxy.html"


def test_build_target_url_appends_anchor() -> None:
    url = _build_target_url(
        _FakeBuilder(), "svg", "_images", "index", "design/proxy", "section-1"
    )

    assert url == "design/proxy.html#section-1"


# ---------------------------------------------------------------------------
# URL escaping: percent-encode, anchor, no bare '#'
# ---------------------------------------------------------------------------


def test_escape_plantuml_url_percent_encodes_brackets_and_spaces() -> None:
    escaped = _escape_plantuml_url("path/a b[c].html")

    assert "[" not in escaped and "]" not in escaped and " " not in escaped


def test_escape_plantuml_url_preserves_fragment_separator() -> None:
    escaped = _escape_plantuml_url("design/proxy.html#my-section")

    assert escaped == "design/proxy.html#my-section"


def test_escape_plantuml_url_encodes_literal_hash_without_fragment() -> None:
    # A '#' that is not the fragment separator must be encoded so it cannot
    # break the PlantUML [[ ]] directive.
    escaped = _escape_plantuml_url("a#b#c")

    # Only the first '#' is treated as the fragment separator; the rest encoded.
    assert escaped.count("#") == 1


# ---------------------------------------------------------------------------
# UML injection + one-URL-per-alias dedup contract
# ---------------------------------------------------------------------------


def test_inject_links_inserts_directive_before_enduml() -> None:
    uml = "@startuml\n[A] --> [B]\n@enduml\n"

    result = _inject_links_into_uml(uml, {"A": "a.html"})

    assert "url of A is [[a.html]]" in result
    assert result.index("url of A") < result.index("@enduml")


def test_inject_links_skips_unsafe_alias() -> None:
    uml = "@startuml\n@enduml\n"

    result = _inject_links_into_uml(uml, {"bad alias!": "x.html"})

    assert "url of" not in result


def test_one_url_per_alias_dedup_contract() -> None:
    # The resolved_links dict in on_doctree_resolved keys by alias, so an
    # alias maps to exactly one URL (last write wins).  Emulate that contract.
    resolved_links: dict[str, str] = {}
    resolved_links["A"] = "first.html"
    resolved_links["A"] = "second.html"

    uml = _inject_links_into_uml("@startuml\n@enduml\n", resolved_links)

    assert uml.count("url of A is") == 1
    assert "[[second.html]]" in uml
