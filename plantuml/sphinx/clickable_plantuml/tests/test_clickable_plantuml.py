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

import clickable_plantuml as clickable_plantuml_module
from clickable_plantuml import (
    _ENV_CALL_LINKS,
    _ENV_ELEMENT_LINKS,
    _ENV_PUML_DOCNAMES,
    _ENV_SOURCE_KEYS,
    _ENV_UNLINKED_NODE_COUNTS_BY_DOC,
    _ENV_WORKSPACE_OFFSET,
    _build_target_url,
    _escape_plantuml_url,
    _inject_links_into_uml,
    _link_message_label,
    _links_digest,
    _load_diagram_links,
    _node_source_key,
    _source_keys,
    on_build_finished,
    on_builder_inited,
    on_config_inited,
    on_doctree_read,
    on_doctree_resolved,
    on_env_merge_info,
    on_env_purge_doc,
)


def _write_links(
    directory: Path,
    element_links: list[dict[str, str]] | None = None,
    call_links: list[dict[str, object]] | None = None,
    version: int = 1,
) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "diagram_links.json").write_text(
        json.dumps(
            {
                "version": version,
                "element_links": element_links or [],
                "call_links": call_links or [],
            }
        ),
        encoding="utf-8",
    )


# ---------------------------------------------------------------------------
# links loading
# ---------------------------------------------------------------------------


def test_load_diagram_links_reads_element_and_call_links(tmp_path: Path) -> None:
    _write_links(
        tmp_path / "de",
        element_links=[{"source": "/pkg/overview.puml", "alias": "Proxy", "target": "pkg/proxy.puml"}],
        call_links=[{"source": "pkg/seq.puml", "line": 3, "method": "Get", "targets": ["pkg/a.puml", "pkg/b.puml"]}],
    )

    element_links, call_links = _load_diagram_links(tmp_path)

    assert element_links == {"pkg/overview.puml": {"Proxy": "pkg/proxy.puml"}}
    assert call_links == {"pkg/seq.puml": [(3, "Get", ["pkg/a.puml", "pkg/b.puml"])]}
    assert _source_keys(element_links, call_links) == {
        "pkg/overview.puml",
        "pkg/proxy.puml",
        "pkg/seq.puml",
        "pkg/a.puml",
        "pkg/b.puml",
    }


def test_load_diagram_links_merges_files_first_wins(tmp_path: Path) -> None:
    _write_links(
        tmp_path / "a",
        element_links=[{"source": "pkg/o.puml", "alias": "P", "target": "pkg/first.puml"}],
    )
    _write_links(
        tmp_path / "b",
        element_links=[
            {"source": "pkg/o.puml", "alias": "P", "target": "pkg/second.puml"},
            {"source": "pkg/o.puml", "alias": "Q", "target": "pkg/q.puml"},
        ],
    )

    element_links, _ = _load_diagram_links(tmp_path)

    assert element_links == {"pkg/o.puml": {"P": "pkg/first.puml", "Q": "pkg/q.puml"}}


def test_load_diagram_links_skips_unknown_version(tmp_path: Path, caplog: pytest.LogCaptureFixture) -> None:
    _write_links(tmp_path, element_links=[{"source": "a.puml", "alias": "A", "target": "b.puml"}], version=2)

    caplog.set_level(logging.WARNING)
    assert _load_diagram_links(tmp_path) == ({}, {})
    assert "expected 1" in caplog.text


def test_builder_inited_without_links_file_loads_nothing(tmp_path: Path) -> None:
    env = SimpleNamespace()
    app = SimpleNamespace(builder=SimpleNamespace(format="html"), srcdir=str(tmp_path), env=env)

    on_builder_inited(app)

    assert not hasattr(env, _ENV_SOURCE_KEYS)


def test_links_digest_changes_with_the_links(tmp_path: Path) -> None:
    empty = _links_digest(tmp_path)
    _write_links(tmp_path / "de", call_links=[{"source": "a.puml", "line": 3, "method": "Get", "targets": ["b.puml"]}])
    with_call = _links_digest(tmp_path)
    _write_links(tmp_path / "de", call_links=[])
    without_call = _links_digest(tmp_path)

    assert len({empty, with_call, without_call}) == 3
    assert _links_digest(tmp_path) == without_call


def test_config_inited_sets_the_links_digest(tmp_path: Path) -> None:
    _write_links(tmp_path / "de")
    config = SimpleNamespace(clickable_plantuml_links_digest="")
    app = SimpleNamespace(srcdir=str(tmp_path))

    on_config_inited(app, config)

    assert config.clickable_plantuml_links_digest == _links_digest(tmp_path)


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
    (real_root / "pkg" / "proxy.puml").write_text("@startuml\n@enduml\n", encoding="utf-8")

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


def test_node_source_key_returns_none_for_inconclusive_basename_overlap() -> None:
    # Two candidates share the node's basename but neither is a unique path
    # suffix match; the removed "best overlap" heuristic used to guess one of
    # them, but that contradicted the "safe over wrong" contract, so this
    # must now return None rather than silently picking a candidate.
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

    assert key is None


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


def test_doctree_read_unknown_node_is_debug_only(
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
    setattr(env, _ENV_SOURCE_KEYS, frozenset({"pkg/a/proxy.puml"}))
    setattr(env, _ENV_WORKSPACE_OFFSET, "/workspace")
    setattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {})

    app = SimpleNamespace(
        builder=SimpleNamespace(format="html"),
        srcdir="/workspace",
        env=env,
    )

    caplog.set_level(logging.DEBUG)
    on_doctree_read(app, doctree)

    assert not any(record.levelno == logging.WARNING for record in caplog.records)
    assert any(
        "has no resolvable source path" in record.getMessage() and record.levelno == logging.DEBUG
        for record in caplog.records
    )
    assert getattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC) == {"index": 1}


def test_build_finished_logs_aggregate_unlinked_count(
    caplog: pytest.LogCaptureFixture,
) -> None:
    env = SimpleNamespace()
    setattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {"a": 2, "b": 1})
    app = SimpleNamespace(builder=SimpleNamespace(format="html"), env=env)

    caplog.set_level(logging.INFO)
    on_build_finished(app, None)

    assert "3 plantuml nodes are in no diagram link" in caplog.text


def test_env_purge_doc_removes_unlinked_count_for_purged_doc() -> None:
    env = SimpleNamespace()
    setattr(env, _ENV_PUML_DOCNAMES, {"pkg/a.puml": ("stale", None)})
    setattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {"stale": 2, "keep": 5})

    on_env_purge_doc(SimpleNamespace(), env, "stale")

    assert getattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC) == {"keep": 5}
    assert getattr(env, _ENV_PUML_DOCNAMES) == {}


def test_env_merge_info_merges_unlinked_counts_from_parallel_workers() -> None:
    env = SimpleNamespace()
    setattr(env, _ENV_PUML_DOCNAMES, {})
    setattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {"a": 1})

    other = SimpleNamespace()
    setattr(other, _ENV_PUML_DOCNAMES, {})
    setattr(other, _ENV_UNLINKED_NODE_COUNTS_BY_DOC, {"b": 4})

    on_env_merge_info(SimpleNamespace(), env, {"b"}, other)

    assert getattr(env, _ENV_UNLINKED_NODE_COUNTS_BY_DOC) == {"a": 1, "b": 4}


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
def test_build_target_url_respects_output_mode(output_format: str, expected: str) -> None:
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


def test_inject_links_quotes_alias_with_special_characters() -> None:
    uml = "@startuml\n@enduml\n"

    result = _inject_links_into_uml(uml, {"bad alias!": "x.html", "a::b::E": "e.html"})

    assert 'url of "bad alias!" is [[x.html]]' in result
    assert 'url of "a::b::E" is [[e.html]]' in result


def test_inject_links_warns_and_skips_alias_that_cannot_be_quoted(
    caplog: pytest.LogCaptureFixture,
) -> None:
    uml = "@startuml\n@enduml\n"

    caplog.set_level(logging.WARNING)
    result = _inject_links_into_uml(uml, {'say "hi"': "x.html", "two\nlines": "y.html"})

    assert "url of" not in result
    assert "cannot be written in a url directive" in caplog.text


def test_inject_links_appends_directives_when_enduml_missing() -> None:
    uml = "@startuml\n[A] --> [B]\n"

    result = _inject_links_into_uml(uml, {"A": "a.html"})

    assert result.endswith("url of A is [[a.html]]")


# ---------------------------------------------------------------------------
# refined call links
# ---------------------------------------------------------------------------

# ---------------------------------------------------------------------------
# call label linking
# ---------------------------------------------------------------------------


def test_link_message_label_wraps_the_label() -> None:
    uml = "@startuml\nA -> B : Get()\n@enduml"

    linked = _link_message_label(uml, 2, "Get", "d.html", None)

    assert linked == "@startuml\nA -> B : [[d.html Get()]]\n@enduml"


def test_link_message_label_adds_tooltip() -> None:
    linked = _link_message_label("A -> B : Get()", 1, "Get", "d.html", "Refined by: a, b")

    assert linked == "A -> B : [[d.html{Refined by: a, b} Get()]]"


def test_link_message_label_accepts_call_operator_and_spaced_arguments() -> None:
    assert _link_message_label("A -> B : operator()(x)", 1, "operator()", "d.html", None) == (
        "A -> B : [[d.html operator()(x)]]"
    )
    assert _link_message_label("A -> B : Get (x)", 1, "Get", "d.html", None) == "A -> B : [[d.html Get (x)]]"


def test_link_message_label_uses_the_colon_outside_quotes() -> None:
    linked = _link_message_label('"a:b" -> B : Get()', 1, "Get", "d.html", None)

    assert linked == '"a:b" -> B : [[d.html Get()]]'


@pytest.mark.parametrize(
    ("uml", "line"),
    [
        ("A -> B : Other()", 1),
        ("A -> B : [[x Get()]]", 1),
        ("A -> B", 1),
        ("A -> B : Get()", 2),
        ("A -> B : GetAll()", 1),
    ],
)
def test_link_message_label_leaves_other_lines_unchanged(uml: str, line: int) -> None:
    assert _link_message_label(uml, line, "Get", "d.html", None) == uml


# ---------------------------------------------------------------------------
# link injection in doctree-resolved
# ---------------------------------------------------------------------------


def _resolve_uml(
    monkeypatch: pytest.MonkeyPatch,
    uml: str,
    element_links: dict[str, dict[str, str]],
    call_links: dict[str, list[tuple[int, str, list[str]]]],
    docnames: dict[str, tuple[str, str | None]],
) -> str:
    """Run ``on_doctree_resolved`` for ``pkg/overview.puml`` and return the injected UML."""

    class _FakePlantumlNode(nodes.Element):
        pass

    monkeypatch.setattr(clickable_plantuml_module, "_get_plantuml_node_class", lambda: _FakePlantumlNode)

    node = _FakePlantumlNode()
    node["filename"] = "overview.puml"
    node["incdir"] = "pkg"
    node["uml"] = uml
    doctree = nodes.document("", "")
    doctree += node

    env = SimpleNamespace()
    setattr(env, _ENV_ELEMENT_LINKS, element_links)
    setattr(env, _ENV_CALL_LINKS, call_links)
    setattr(env, _ENV_SOURCE_KEYS, _source_keys(element_links, call_links) | {"pkg/overview.puml"})
    setattr(env, _ENV_WORKSPACE_OFFSET, "/workspace")
    setattr(env, _ENV_PUML_DOCNAMES, docnames)

    class _ResolveBuilder:
        format = "html"
        imagedir = "_images"

        def get_relative_uri(self, from_docname: str, to_docname: str) -> str:
            _ = from_docname
            return f"{to_docname}.html"

    app = SimpleNamespace(
        builder=_ResolveBuilder(),
        config=SimpleNamespace(plantuml_output_format="svg"),
        srcdir="/workspace",
        env=env,
    )
    on_doctree_resolved(app, doctree, "index")
    return node["uml"]


def test_doctree_resolved_injects_element_link(monkeypatch: pytest.MonkeyPatch) -> None:
    uml = _resolve_uml(
        monkeypatch,
        "@startuml\n[Proxy]\n@enduml\n",
        {"pkg/overview.puml": {"Proxy": "pkg/proxy.puml"}},
        {},
        {"pkg/proxy.puml": ("design/proxy", "proxy-section")},
    )

    assert "url of Proxy is [[design/proxy.html#proxy-section]]" in uml


def test_doctree_resolved_skips_target_outside_the_documents(monkeypatch: pytest.MonkeyPatch) -> None:
    uml = _resolve_uml(
        monkeypatch,
        "@startuml\n[Proxy]\n@enduml\n",
        {"pkg/overview.puml": {"Proxy": "pkg/proxy.puml"}},
        {},
        {},
    )

    assert "url of" not in uml


_OVERVIEW_UML = "@startuml\n\nApp -> seooc : Get()\nseooc -> Ext : Log()\n@enduml"


def test_doctree_resolved_links_call_to_its_detail(monkeypatch: pytest.MonkeyPatch) -> None:
    uml = _resolve_uml(
        monkeypatch,
        _OVERVIEW_UML,
        {},
        {"pkg/overview.puml": [(3, "Get", ["pkg/component.puml"])]},
        {"pkg/component.puml": ("design/component", None)},
    )

    assert "App -> seooc : [[design/component.html Get()]]" in uml
    assert "seooc -> Ext : Log()" in uml


def test_doctree_resolved_names_all_details_in_tooltip(monkeypatch: pytest.MonkeyPatch) -> None:
    uml = _resolve_uml(
        monkeypatch,
        _OVERVIEW_UML,
        {},
        {"pkg/overview.puml": [(3, "Get", ["pkg/a.puml", "pkg/b.puml"])]},
        {"pkg/a.puml": ("design/a", None), "pkg/b.puml": ("design/b", None)},
    )

    assert "App -> seooc : [[design/a.html{Refined by: a, b} Get()]]" in uml


def test_doctree_resolved_keeps_stale_call_line(monkeypatch: pytest.MonkeyPatch) -> None:
    uml = _resolve_uml(
        monkeypatch,
        _OVERVIEW_UML,
        {},
        {"pkg/overview.puml": [(4, "Get", ["pkg/component.puml"])]},
        {"pkg/component.puml": ("design/component", None)},
    )

    assert uml == _OVERVIEW_UML
