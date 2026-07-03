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
import logging
import os
from pathlib import Path
from types import SimpleNamespace

import pytest
from docutils import nodes
from sphinx.errors import ExtensionError

import clickable_plantuml as clickable_plantuml_module
from clickable_plantuml import (
    _ENV_DEF_INDEX,
    _ENV_IDMAP_BY_SOURCE,
    _ENV_NO_IDMAP_NODE_COUNT,
    _ENV_PUML_DOCNAMES,
    _ENV_SOURCE_KEYS,
    _ENV_WORKSPACE_OFFSET,
    _build_target_url,
    _common_prefix_length,
    _escape_plantuml_url,
    _inject_links_into_uml,
    _load_idmap_files,
    _node_source_key,
    on_build_finished,
    on_doctree_read,
    on_doctree_resolved,
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
        "/pkg/a/proxy.puml",
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
# node source-key resolution (strict exact matching)
# ---------------------------------------------------------------------------


def test_node_source_key_same_basename_stays_distinct() -> None:
    node = {
        "filename": "proxy.puml",
        "incdir": "pkg/b",
    }

    key = _node_source_key(
        node,
        "/workspace",
        "/workspace",
        {"pkg/a/proxy.puml", "pkg/b/proxy.puml"},
    )

    assert key == "pkg/b/proxy.puml"


@pytest.mark.parametrize(
    "node",
    [
        {"filename": "stray.puml", "incdir": "pkg/b"},
        {"incdir": "pkg/b"},
    ],
)
def test_node_source_key_returns_none_for_unresolvable_nodes(
    node: dict[str, str],
) -> None:
    key = _node_source_key(
        node,
        "/workspace",
        "/workspace",
        {"pkg/a/proxy.puml"},
    )

    assert key is None


def test_node_source_key_resolves_symlinked_srcdir(tmp_path: Path) -> None:
    real_root = tmp_path / "real_root"
    (real_root / "pkg").mkdir(parents=True)
    (real_root / "pkg" / "proxy.puml").write_text(
        "@startuml\n@enduml\n", encoding="utf-8"
    )

    srcdir_link = tmp_path / "srcdir_link"
    os.symlink(real_root, srcdir_link)

    node = {
        "filename": "proxy.puml",
        "incdir": "pkg",
    }

    key = _node_source_key(
        node,
        str(srcdir_link),
        str(real_root),
        {"pkg/proxy.puml"},
    )

    assert key == "pkg/proxy.puml"


def test_node_source_key_resolves_unique_suffix_match() -> None:
    node = {
        "filename": "static_design.puml",
        "incdir": "architectural_design",
    }

    key = _node_source_key(
        node,
        "/workspace/synthetic_src",
        "/workspace/synthetic_src",
        {
            "bazel/rules/rules_score/examples/seooc/safety_software_seooc_example_index/architectural_design/static_design.puml",
        },
    )

    assert (
        key
        == "bazel/rules/rules_score/examples/seooc/safety_software_seooc_example_index/architectural_design/static_design.puml"
    )


def test_node_source_key_resolves_absolute_incdir_unique_suffix() -> None:
    node = {
        "filename": "static_design.puml",
        "incdir": "/tmp/sandbox/work/src/architectural_design",
    }

    key = _node_source_key(
        node,
        "/workspace/synthetic_src",
        "/workspace/synthetic_src",
        {
            "architectural_design/static_design.puml",
        },
    )

    assert key == "architectural_design/static_design.puml"


def test_node_source_key_resolves_best_basename_overlap_match() -> None:
    node = {
        "filename": "static_design.puml",
        "incdir": "architectural_design",
    }

    key = _node_source_key(
        node,
        "/workspace/seooc/safety_software_seooc_example_doc",
        "/workspace/seooc/safety_software_seooc_example_doc",
        {
            "bazel/rules/rules_score/examples/seooc/design/static_design.puml",
            "bazel/rules/rules_score/examples/some_other_library/static_design.puml",
        },
    )

    assert key == "bazel/rules/rules_score/examples/seooc/design/static_design.puml"


def test_node_source_key_returns_none_for_tied_basename_overlap() -> None:
    node = {
        "filename": "same.puml",
        "incdir": "x",
    }

    key = _node_source_key(
        node,
        "/workspace/root",
        "/workspace/root",
        {
            "a/same.puml",
            "b/same.puml",
        },
    )

    assert key is None


def test_doctree_read_non_idmap_backed_node_is_debug_only(
    caplog: pytest.LogCaptureFixture,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class _FakePlantumlNode(nodes.Element):
        pass

    monkeypatch.setattr(
        clickable_plantuml_module,
        "_get_plantuml_node_class",
        lambda: _FakePlantumlNode,
    )

    node = _FakePlantumlNode()
    node["filename"] = "stray.puml"
    node["incdir"] = "pkg/b"
    doctree = nodes.document("", "")
    doctree += node

    env = SimpleNamespace(docname="index")
    setattr(env, _ENV_IDMAP_BY_SOURCE, {"pkg/a/proxy.puml": {}})
    setattr(env, _ENV_SOURCE_KEYS, frozenset({"pkg/a/proxy.puml"}))
    setattr(env, _ENV_WORKSPACE_OFFSET, "/workspace")
    setattr(env, _ENV_NO_IDMAP_NODE_COUNT, 0)

    app = SimpleNamespace(
        builder=SimpleNamespace(format="html"),
        srcdir="/workspace",
        env=env,
    )

    caplog.set_level(logging.DEBUG)
    on_doctree_read(app, doctree)

    assert not any(record.levelno == logging.WARNING for record in caplog.records)
    assert any(
        "has no resolvable source path" in record.getMessage()
        and record.levelno == logging.DEBUG
        for record in caplog.records
    )
    assert getattr(env, _ENV_NO_IDMAP_NODE_COUNT) == 1


def test_build_finished_logs_aggregate_no_idmap_count(
    caplog: pytest.LogCaptureFixture,
) -> None:
    env = SimpleNamespace()
    setattr(env, _ENV_NO_IDMAP_NODE_COUNT, 3)
    app = SimpleNamespace(builder=SimpleNamespace(format="html"), env=env)

    caplog.set_level(logging.INFO)
    on_build_finished(app, None)

    assert "3 plantuml nodes had no idmap (not linkable)" in caplog.text


def test_doctree_resolved_injects_link_directives(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class _FakePlantumlNode(nodes.Element):
        pass

    monkeypatch.setattr(
        clickable_plantuml_module,
        "_get_plantuml_node_class",
        lambda: _FakePlantumlNode,
    )

    node = _FakePlantumlNode()
    node["filename"] = "overview.puml"
    node["incdir"] = "pkg"
    node["uml"] = "@startuml\n[Proxy]\n@enduml\n"

    doctree = nodes.document("", "")
    doctree += node

    env = SimpleNamespace()
    setattr(
        env,
        _ENV_IDMAP_BY_SOURCE,
        {
            "pkg/overview.puml": {
                "references": [{"alias": "Proxy", "id": "pkg.Proxy"}],
            }
        },
    )
    setattr(env, _ENV_DEF_INDEX, {"pkg.Proxy": ["pkg/proxy.puml"]})
    setattr(env, _ENV_SOURCE_KEYS, frozenset({"pkg/overview.puml", "pkg/proxy.puml"}))
    setattr(env, _ENV_WORKSPACE_OFFSET, "/workspace")
    setattr(
        env, _ENV_PUML_DOCNAMES, {"pkg/proxy.puml": ("design/proxy", "proxy-section")}
    )

    class _FakeBuilder:
        format = "html"
        imagedir = "_images"

        def get_relative_uri(self, from_docname: str, to_docname: str) -> str:
            _ = from_docname
            return f"{to_docname}.html"

    app = SimpleNamespace(
        builder=_FakeBuilder(),
        config=SimpleNamespace(plantuml_output_format="svg"),
        srcdir="/workspace",
        env=env,
    )

    on_doctree_resolved(app, doctree, "index")

    assert "url of Proxy is [[design/proxy.html#proxy-section]]" in node["uml"]


# ---------------------------------------------------------------------------
# reference resolution and tiebreak behavior
# ---------------------------------------------------------------------------


def test_resolve_definer_prefers_fqn_over_alias() -> None:
    definition_index = {
        "Proxy": ["pkg/alias_hit.puml"],
        "pkg.Proxy": ["pkg/fqn_hit.puml"],
    }

    target = _resolve_definer("Proxy", "pkg.Proxy", "pkg/src.puml", definition_index)

    assert target == "pkg/fqn_hit.puml"


def test_resolve_definer_falls_back_to_alias_when_fqn_missing() -> None:
    definition_index = {
        "Proxy": ["pkg/alias_hit.puml"],
    }

    target = _resolve_definer("Proxy", "pkg.Proxy", "pkg/src.puml", definition_index)

    assert target == "pkg/alias_hit.puml"


def test_resolve_definer_skips_self_link() -> None:
    definition_index = {
        "Proxy": ["pkg/src.puml"],
    }

    target = _resolve_definer("Proxy", "Proxy", "pkg/src.puml", definition_index)

    assert target is None


def test_resolve_definer_tie_returns_none_and_warns(
    caplog: pytest.LogCaptureFixture,
) -> None:
    definition_index = {
        "Proxy": ["a/one.puml", "b/two.puml"],
    }

    caplog.set_level(logging.WARNING)
    target = _resolve_definer("Proxy", "Proxy", "pkg/src.puml", definition_index)

    assert target is None
    assert "ambiguous definition" in caplog.text


def test_common_prefix_length_requires_canonical_keys() -> None:
    with pytest.raises(ValueError, match="non-canonical source key"):
        _common_prefix_length("/abs/a.puml", "pkg/b.puml")


# ---------------------------------------------------------------------------
# URL building and escaping
# ---------------------------------------------------------------------------


class _FakeBuilder:
    def get_target_uri(self, docname: str) -> str:
        return f"{docname}.html"

    def get_relative_uri(self, from_docname: str, to_docname: str) -> str:
        _ = from_docname
        return f"{to_docname}.html"


@pytest.mark.parametrize(
    ("output_format", "expected"),
    [
        ("svg_obj", "../design/proxy.html"),
        ("svg", "design/proxy.html"),
    ],
)
def test_build_target_url_respects_output_mode(
    output_format: str, expected: str
) -> None:
    url = _build_target_url(
        _FakeBuilder(),
        output_format,
        "_images",
        "index",
        "design/proxy",
        None,
    )

    assert url == expected


def test_build_target_url_appends_anchor() -> None:
    url = _build_target_url(
        _FakeBuilder(),
        "svg",
        "_images",
        "index",
        "design/proxy",
        "section-1",
    )

    assert url == "design/proxy.html#section-1"


@pytest.mark.parametrize(
    ("raw", "expected_fragment_count", "must_contain", "must_not_contain"),
    [
        ("design/proxy.html#my-section", 1, ["#my-section"], []),
        ("a#b#c", 1, ["%23"], []),
        ("design/has]]end.html", 0, ["%5D"], ["]]"]),
        ("path/a b[c].html", 0, [], [" ", "[", "]"]),
    ],
)
def test_escape_plantuml_url_core_safety(
    raw: str,
    expected_fragment_count: int,
    must_contain: list[str],
    must_not_contain: list[str],
) -> None:
    escaped = _escape_plantuml_url(raw)

    assert escaped.count("#") == expected_fragment_count
    for token in must_contain:
        assert token in escaped
    for token in must_not_contain:
        assert token not in escaped


def test_inject_links_inserts_directive_before_enduml() -> None:
    uml = "@startuml\n[A] --> [B]\n@enduml\n"

    result = _inject_links_into_uml(uml, {"A": "a.html"})

    assert "url of A is [[a.html]]" in result
    assert result.index("url of A") < result.index("@enduml")


def test_inject_links_skips_unsafe_alias() -> None:
    uml = "@startuml\n@enduml\n"

    result = _inject_links_into_uml(uml, {"bad alias!": "x.html"})

    assert "url of" not in result


def test_inject_links_appends_directives_when_enduml_missing() -> None:
    uml = "@startuml\n[A] --> [B]\n"

    result = _inject_links_into_uml(uml, {"A": "a.html"})

    assert result.endswith("url of A is [[a.html]]")


def test_one_url_per_alias_dedup_contract() -> None:
    # The resolved_links dict in on_doctree_resolved keys by alias, so an
    # alias maps to exactly one URL (last write wins).  Emulate that contract.
    resolved_links: dict[str, str] = {}
    resolved_links["A"] = "first.html"
    resolved_links["A"] = "second.html"

    uml = _inject_links_into_uml("@startuml\n@enduml\n", resolved_links)

    assert uml.count("url of A is") == 1
    assert "[[second.html]]" in uml
