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
import json
from pathlib import Path

import pytest

from test_case_coverage.read_gtest_lobster import (
    TestRecord,
    _parse_gwt,
    read_gtest_lobster,
    read_req_metadata_from_lobster_files,
    resolve_path,
    scan_gtest_lobster,
)


def _write_gtest_lobster(path: Path, items: list[tuple[str, list[str], str]]) -> Path:
    """Write a minimal gtest.lobster JSON file.

    Args:
        path: Directory to write the file in.
        items: List of (uid, req_ids, text) tuples.
    """
    data = {
        "schema": "lobster-act-trace",
        "version": 3,
        "generator": "lobster_gtest",
        "data": [
            {
                "tag": f"gtest {uid}",
                "location": {"kind": "void"},
                "name": uid,
                "messages": [],
                "just_up": [],
                "just_down": [],
                "just_global": [],
                "refs": [f"req {r}" for r in req_ids],
                "framework": "GoogleTest",
                "kind": "test",
                "text": text or None,
                "status": "ok",
            }
            for uid, req_ids, text in items
        ],
    }
    p = path / "gtest.lobster"
    p.write_text(json.dumps(data), encoding="utf-8")
    return p


def test_single_item_basic(tmp_path):
    p = _write_gtest_lobster(
        tmp_path, [("Suite:TestName", ["Req.A"], ":Given: g\n:When: w\n:Then: t")]
    )
    records = read_gtest_lobster(p)
    assert len(records) == 1
    r = records[0]
    assert r.uid == "Suite:TestName"
    assert r.lobster_traces == ["Req.A"]
    assert r.given == "g"
    assert r.when == "w"
    assert r.then == "t"


def test_item_captures_raw_gtest_tag(tmp_path):
    """gtest_tag preserves the raw "Suite:TestName" tag so lobster_generator.py
    can build a Tracing_Tag("gtest", ...) cross-reference to the source item.
    """
    p = _write_gtest_lobster(tmp_path, [("Suite:TestName", ["Req.A"], "")])
    r = read_gtest_lobster(p)[0]
    assert r.gtest_tag == "Suite:TestName"


def test_item_with_no_req_refs_is_skipped(tmp_path):
    p = _write_gtest_lobster(tmp_path, [("Suite:TestName", [], "")])
    records = read_gtest_lobster(p)
    assert records == []


def test_non_gtest_tags_are_skipped(tmp_path):
    data = {
        "schema": "lobster-act-trace",
        "version": 3,
        "generator": "test",
        "data": [
            {
                "tag": "req SomeReq",
                "location": {"kind": "void"},
                "name": "SomeReq",
                "messages": [],
                "just_up": [],
                "just_down": [],
                "just_global": [],
                "refs": [],
                "framework": "trlc",
                "kind": "requirement",
                "text": None,
                "status": "ok",
            }
        ],
    }
    p = tmp_path / "mixed.lobster"
    p.write_text(json.dumps(data), encoding="utf-8")
    records = read_gtest_lobster(p)
    assert records == []


def test_multiple_req_refs(tmp_path):
    p = _write_gtest_lobster(tmp_path, [("S:T", ["Req.A", "Req.B"], "")])
    records = read_gtest_lobster(p)
    assert len(records) == 1
    assert records[0].lobster_traces == ["Req.A", "Req.B"]


def test_empty_spec_stored_as_empty_string(tmp_path):
    p = _write_gtest_lobster(tmp_path, [("S:T", ["Req.A"], "")])
    records = read_gtest_lobster(p)
    assert records[0].given == ""
    assert records[0].when == ""
    assert records[0].then == ""


def test_none_text_stored_as_empty_string(tmp_path):
    """Items with text=null (no GWT annotation) store given/when/then as empty strings."""
    data = {
        "schema": "lobster-act-trace",
        "version": 3,
        "generator": "test",
        "data": [
            {
                "tag": "gtest S:T",
                "location": {"kind": "void"},
                "name": "S:T",
                "messages": [],
                "just_up": [],
                "just_down": [],
                "just_global": [],
                "refs": ["req Req.A"],
                "framework": "GoogleTest",
                "kind": "test",
                "text": None,
                "status": "ok",
            }
        ],
    }
    p = tmp_path / "g.lobster"
    p.write_text(json.dumps(data), encoding="utf-8")
    records = read_gtest_lobster(p)
    assert records[0].given == ""
    assert records[0].when == ""
    assert records[0].then == ""


def test_invalid_json_raises_value_error(tmp_path):
    p = tmp_path / "bad.lobster"
    p.write_text("{invalid json", encoding="utf-8")
    with pytest.raises(ValueError, match="Cannot parse gtest.lobster"):
        read_gtest_lobster(p)


# ---------------------------------------------------------------------------
# resolve_path
# ---------------------------------------------------------------------------


def test_resolve_path_absolute_existing(tmp_path):
    f = tmp_path / "x.txt"
    f.write_text("hi")
    assert resolve_path(str(f)) == f


def test_resolve_path_relative_existing(tmp_path, monkeypatch):
    f = tmp_path / "x.txt"
    f.write_text("hi")
    monkeypatch.chdir(tmp_path)
    result = resolve_path("x.txt")
    assert result.exists()


def test_resolve_path_nonexistent_returns_path():
    p = resolve_path("/no/such/file.lobster")
    assert str(p) == "/no/such/file.lobster"


def test_resolve_path_via_runfiles_dir(tmp_path, monkeypatch):
    runfiles = tmp_path / "runfiles"
    (runfiles / "pkg").mkdir(parents=True)
    target = runfiles / "pkg" / "x.txt"
    target.write_text("hi")
    monkeypatch.chdir(tmp_path)  # so the plain-CWD lookup does not match first
    monkeypatch.setenv("RUNFILES_DIR", str(runfiles))
    result = resolve_path("pkg/x.txt")
    assert result == target


def test_resolve_path_runfiles_dir_set_but_file_missing_falls_through(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("RUNFILES_DIR", str(tmp_path / "nonexistent_runfiles"))
    result = resolve_path("missing/x.txt")
    assert str(result) == "missing/x.txt"


# ---------------------------------------------------------------------------
# _parse_gwt
# ---------------------------------------------------------------------------


def test_parse_gwt_basic():
    assert _parse_gwt(":Given: g\n:When: w\n:Then: t") == ("g", "w", "t")


def test_parse_gwt_missing_trailing_space():
    """A key with no space before the value (or no value at all) must not crash."""
    assert _parse_gwt(":Given:g\n:When:\n:Then: t") == ("g", "", "t")


def test_parse_gwt_multiline_value_is_joined():
    text = ":Given: first line\nsecond line\n:When: w\n:Then: t"
    given, when, then = _parse_gwt(text)
    assert given == "first line second line"
    assert when == "w"
    assert then == "t"


def test_parse_gwt_duplicate_key_last_wins():
    text = ":Given: first\n:Given: second\n:When: w\n:Then: t"
    given, _when, _then = _parse_gwt(text)
    assert given == "second"


def test_parse_gwt_blank_lines_ignored():
    text = ":Given: g\n\n:When: w\n\n:Then: t"
    assert _parse_gwt(text) == ("g", "w", "t")


def test_parse_gwt_empty_text():
    assert _parse_gwt("") == ("", "", "")


def test_parse_gwt_text_before_any_key_ignored():
    text = "preamble noise\n:Given: g\n:When: w\n:Then: t"
    assert _parse_gwt(text) == ("g", "w", "t")


# ---------------------------------------------------------------------------
# read_req_ids_from_lobster_files
# ---------------------------------------------------------------------------


def _write_req_lobster(
    path: Path,
    req_ids: list[str],
    kind: str = "CompReq",
    version: str = "1",
    text: str = "",
) -> Path:
    data = {
        "schema": "lobster-req-trace",
        "version": 4,
        "generator": "test",
        "data": [
            {
                "tag": f"req {rid}@{version}",
                "name": rid,
                "location": {"kind": "void"},
                "messages": [],
                "just_up": [],
                "just_down": [],
                "just_global": [],
                "framework": "TRLC",
                "kind": kind,
                "text": text or None,
                "status": None,
            }
            for rid in req_ids
        ],
    }
    p = path / "reqs.lobster"
    p.write_text(json.dumps(data), encoding="utf-8")
    return p


def _write_manifest(tmp_path: Path, lobster_path: Path) -> Path:
    m = tmp_path / "manifest.txt"
    m.write_text(str(lobster_path) + "\n", encoding="utf-8")
    return m


def test_read_req_ids_basic(tmp_path):
    lobster = _write_req_lobster(tmp_path, ["P.A", "P.B"])
    manifest = _write_manifest(tmp_path, lobster)
    metadata = read_req_metadata_from_lobster_files(manifest)
    assert [m.id for m in metadata] == ["P.A", "P.B"]


def test_read_req_ids_sorted(tmp_path):
    lobster = _write_req_lobster(tmp_path, ["P.Z", "P.A"])
    manifest = _write_manifest(tmp_path, lobster)
    assert [m.id for m in read_req_metadata_from_lobster_files(manifest)] == [
        "P.A",
        "P.Z",
    ]


def test_read_req_ids_deduplicated(tmp_path):
    # A real lobster-req-trace file can't contain the same tag twice (lobster
    # itself rejects that as a duplicate definition); dedup only matters when
    # the same requirement is reachable via two different manifest entries
    # (e.g. two lobster files listing the same requirement).
    dir_a = tmp_path / "a"
    dir_a.mkdir()
    dir_b = tmp_path / "b"
    dir_b.mkdir()
    lobster_a = _write_req_lobster(dir_a, ["P.A"])
    lobster_b = _write_req_lobster(dir_b, ["P.A"])
    manifest = tmp_path / "manifest.txt"
    manifest.write_text(f"{lobster_a}\n{lobster_b}\n", encoding="utf-8")
    assert len(read_req_metadata_from_lobster_files(manifest)) == 1


def test_read_req_version_extracted(tmp_path):
    lobster = _write_req_lobster(tmp_path, ["P.A"], version="3")
    manifest = _write_manifest(tmp_path, lobster)
    meta = read_req_metadata_from_lobster_files(manifest)
    assert meta[0].version == "3"


def test_read_req_description_extracted(tmp_path):
    lobster = _write_req_lobster(tmp_path, ["P.A"], text="The system shall do X.")
    manifest = _write_manifest(tmp_path, lobster)
    meta = read_req_metadata_from_lobster_files(manifest)
    assert meta[0].description == "The system shall do X."


def test_feat_req_items_excluded(tmp_path):
    """FeatReq items in the lobster file must not appear in the output."""
    lobster = _write_req_lobster(tmp_path, ["Feat.001"], kind="FeatReq")
    manifest = _write_manifest(tmp_path, lobster)
    assert read_req_metadata_from_lobster_files(manifest) == []


def test_mixed_kinds_only_comp_req_returned(tmp_path):
    """CompReq returned, FeatReq silently skipped."""
    import json as _json

    data = {
        "schema": "lobster-req-trace",
        "version": 4,
        "generator": "test",
        "data": [
            {
                "tag": "req C.001@1",
                "name": "C.001",
                "location": {"kind": "void"},
                "messages": [],
                "just_up": [],
                "just_down": [],
                "just_global": [],
                "framework": "TRLC",
                "kind": "CompReq",
                "text": None,
                "status": None,
            },
            {
                "tag": "req F.001@1",
                "name": "F.001",
                "location": {"kind": "void"},
                "messages": [],
                "just_up": [],
                "just_down": [],
                "just_global": [],
                "framework": "TRLC",
                "kind": "FeatReq",
                "text": None,
                "status": None,
            },
        ],
    }
    p = tmp_path / "mixed.lobster"
    p.write_text(_json.dumps(data), encoding="utf-8")
    manifest = tmp_path / "manifest.txt"
    manifest.write_text(str(p) + "\n", encoding="utf-8")
    meta = read_req_metadata_from_lobster_files(manifest)
    assert [m.id for m in meta] == ["C.001"]


# ---------------------------------------------------------------------------
# scan_gtest_lobster
# ---------------------------------------------------------------------------


def test_scan_groups_by_req(tmp_path):
    gtest = _write_gtest_lobster(
        tmp_path, [("Suite:T", ["P.R"], ":Given: g\n:When: w\n:Then: t")]
    )
    by_req = scan_gtest_lobster(gtest, ["P.R"])
    assert len(by_req["P.R"]) == 1
    rec = by_req["P.R"][0]
    assert rec.given == "g"
    assert rec.when == "w"
    assert rec.then == "t"


def test_scan_uid_no_package(tmp_path):
    gtest = _write_gtest_lobster(tmp_path, [("Suite:T", ["P.R"], "")])
    by_req = scan_gtest_lobster(gtest, ["P.R"])
    assert by_req["P.R"][0].uid == "Suite:T"


def test_scan_uid_with_package(tmp_path):
    gtest = _write_gtest_lobster(tmp_path, [("Suite:T", ["P.R"], "")])
    by_req = scan_gtest_lobster(gtest, ["P.R"], package="//score/msg")
    assert by_req["P.R"][0].uid == "//score/msg/Suite:T"


def test_scan_gtest_tag_survives_package_prefix(tmp_path):
    """gtest_tag must stay the raw, unprefixed tag even once uid is prefixed,
    so it keeps matching the underlying gtest item's Tracing_Tag.
    """
    gtest = _write_gtest_lobster(tmp_path, [("Suite:T", ["P.R"], "")])
    by_req = scan_gtest_lobster(gtest, ["P.R"], package="//score/msg")
    rec = by_req["P.R"][0]
    assert rec.uid == "//score/msg/Suite:T"
    assert rec.gtest_tag == "Suite:T"


def test_scan_skips_unknown_reqs(tmp_path):
    gtest = _write_gtest_lobster(tmp_path, [("Suite:T", ["P.Unknown"], "")])
    by_req = scan_gtest_lobster(gtest, ["P.R"])
    assert by_req["P.R"] == []


def test_scan_multi_trace_appears_in_each(tmp_path):
    gtest = _write_gtest_lobster(tmp_path, [("Suite:T", ["P.A", "P.B"], "")])
    by_req = scan_gtest_lobster(gtest, ["P.A", "P.B"])
    assert len(by_req["P.A"]) == 1
    assert len(by_req["P.B"]) == 1


if __name__ == "__main__":
    import sys

    sys.exit(pytest.main([__file__, "-v"]))
