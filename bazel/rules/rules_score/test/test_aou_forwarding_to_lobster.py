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
"""Tests for aou_forwarding_to_lobster."""

import tempfile
import unittest

import yaml
from lobster.common.items import Requirement, Tracing_Tag
from lobster.common.location import Void_Reference

from aou_forwarding_to_lobster import (
    build_forwarded_markers,
    filter_forwarded_aous,
    load_lobster_items,
    parse_forwarding_yaml,
)


def _req(tag: str, name: str) -> Requirement:
    """Build a minimal Requirement item with the given 'req Pkg.Name[@ver]' tag."""
    namespace, rest = tag.split(" ", 1)
    return Requirement(
        tag=Tracing_Tag.from_text(namespace, rest),
        location=Void_Reference(),
        framework="TRLC",
        kind="AoU",
        name=name,
    )


class TestParseForwardingYaml(unittest.TestCase):
    """Tests for parse_forwarding_yaml."""

    def _write_yaml(self, data: dict) -> str:
        f = tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False)
        yaml.dump(data, f)
        f.close()
        return f.name

    def test_valid_yaml(self) -> None:
        path = self._write_yaml(
            {
                "forwarded_aous": [
                    {"aou_id": "Pkg.AoU1", "justification": "reason"},
                ]
            }
        )
        result = parse_forwarding_yaml(path)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["aou_id"], "Pkg.AoU1")
        self.assertEqual(result[0]["justification"], "reason")

    def test_missing_forwarded_aous_key(self) -> None:
        path = self._write_yaml({"wrong_key": []})
        with self.assertRaises(SystemExit):
            parse_forwarding_yaml(path)

    def test_missing_aou_id(self) -> None:
        path = self._write_yaml({"forwarded_aous": [{"justification": "r"}]})
        with self.assertRaises(SystemExit):
            parse_forwarding_yaml(path)

    def test_missing_justification(self) -> None:
        path = self._write_yaml({"forwarded_aous": [{"aou_id": "Foo.Bar"}]})
        with self.assertRaises(SystemExit):
            parse_forwarding_yaml(path)

    def test_multiple_entries(self) -> None:
        path = self._write_yaml(
            {
                "forwarded_aous": [
                    {"aou_id": "A.B", "justification": "r1"},
                    {"aou_id": "C.D", "justification": "r2"},
                ]
            }
        )
        result = parse_forwarding_yaml(path)
        self.assertEqual(len(result), 2)


class TestLoadLobsterItems(unittest.TestCase):
    """Tests for load_lobster_items."""

    def _write_lobster(self, tags: list[str]) -> str:
        from lobster.common.io import lobster_write

        items = [_req(tag, tag.split(" ", 1)[1].split("@")[0]) for tag in tags]
        f = tempfile.NamedTemporaryFile(mode="w", suffix=".lobster", delete=False)
        lobster_write(f, Requirement, "test", items)
        f.close()
        return f.name

    def test_loads_items(self) -> None:
        path = self._write_lobster(["req Pkg.AoU1", "req Pkg.AoU2"])
        loaded = load_lobster_items([path])
        self.assertEqual(len(loaded), 2)
        self.assertEqual(str(loaded[0].tag), "req Pkg.AoU1")

    def test_multiple_files(self) -> None:
        path1 = self._write_lobster(["req A.B"])
        path2 = self._write_lobster(["req C.D"])
        loaded = load_lobster_items([path1, path2])
        self.assertEqual(len(loaded), 2)

    def test_empty_data(self) -> None:
        path = self._write_lobster([])
        loaded = load_lobster_items([path])
        self.assertEqual(loaded, [])


class TestFilterForwardedAous(unittest.TestCase):
    """Tests for filter_forwarded_aous."""

    def test_filters_correctly(self) -> None:
        items = [_req("req Pkg.AoU1", "AoU1"), _req("req Pkg.AoU2", "AoU2")]
        entries = [{"aou_id": "Pkg.AoU1", "justification": "reason"}]
        filtered = filter_forwarded_aous(entries, items)
        self.assertEqual(len(filtered), 1)
        self.assertEqual(str(filtered[0].tag), "req Pkg.AoU1")

    def test_multiple_filters(self) -> None:
        items = [
            _req("req A.B", "B"),
            _req("req C.D", "D"),
            _req("req E.F", "F"),
        ]
        entries = [
            {"aou_id": "A.B", "justification": "r1"},
            {"aou_id": "E.F", "justification": "r2"},
        ]
        filtered = filter_forwarded_aous(entries, items)
        self.assertEqual(len(filtered), 2)

    def test_nonexistent_aou_id_raises(self) -> None:
        items = [_req("req Pkg.AoU1", "AoU1")]
        entries = [{"aou_id": "NonExistent.Foo", "justification": "reason"}]
        with self.assertRaises(SystemExit):
            filter_forwarded_aous(entries, items)

    def test_versioned_tag_matches_base_id(self) -> None:
        """lobster-trlc generates versioned tags like 'req Pkg.Name@1'."""
        items = [
            _req("req Pkg.AoU1@1", "AoU1"),
            _req("req Pkg.AoU2@3", "AoU2"),
        ]
        entries = [{"aou_id": "Pkg.AoU1", "justification": "reason"}]
        filtered = filter_forwarded_aous(entries, items)
        self.assertEqual(len(filtered), 1)
        self.assertEqual(str(filtered[0].tag), "req Pkg.AoU1@1")

    def test_versioned_tag_matches_full_id(self) -> None:
        """Full versioned ID should also work."""
        items = [_req("req Pkg.AoU1@2", "AoU1")]
        entries = [{"aou_id": "Pkg.AoU1@2", "justification": "reason"}]
        filtered = filter_forwarded_aous(entries, items)
        self.assertEqual(len(filtered), 1)


class TestBuildForwardedMarkers(unittest.TestCase):
    """Tests for build_forwarded_markers."""

    def test_builds_one_marker_per_entry(self) -> None:
        items = [_req("req Pkg.AoU1@1", "AoU1"), _req("req Pkg.AoU2@1", "AoU2")]
        entries = [
            {"aou_id": "Pkg.AoU1", "justification": "reason 1"},
            {"aou_id": "Pkg.AoU2", "justification": "reason 2"},
        ]
        markers = build_forwarded_markers(entries, items, "aou_forwarding.yaml")
        self.assertEqual(len(markers), 2)

    def test_marker_has_distinct_tag_and_refs_original(self) -> None:
        """The marker's tag must not collide with the original item's tag
        (so it can coexist with the "Received AoUs" level in the same
        report), but its refs must point at the original tag."""
        items = [_req("req Pkg.AoU1@1", "AoU1")]
        entries = [{"aou_id": "Pkg.AoU1", "justification": "reason"}]
        markers = build_forwarded_markers(entries, items, "aou_forwarding.yaml")
        marker = markers[0]
        self.assertNotEqual(str(marker.tag), "req Pkg.AoU1@1")
        self.assertEqual(str(marker.tag), "req Pkg.AoU1__forwarded")
        self.assertEqual(
            [str(ref) for ref in marker.unresolved_references],
            ["req Pkg.AoU1@1"],
        )

    def test_marker_uses_justification_as_text(self) -> None:
        items = [_req("req Pkg.AoU1@1", "AoU1")]
        entries = [{"aou_id": "Pkg.AoU1", "justification": "must be handled downstream"}]
        markers = build_forwarded_markers(entries, items, "aou_forwarding.yaml")
        self.assertEqual(markers[0].text, "must be handled downstream")

    def test_marker_kind_and_framework(self) -> None:
        items = [_req("req Pkg.AoU1@1", "AoU1")]
        entries = [{"aou_id": "Pkg.AoU1", "justification": "reason"}]
        markers = build_forwarded_markers(entries, items, "aou_forwarding.yaml")
        self.assertEqual(markers[0].kind, "ForwardedAoU")
        self.assertEqual(markers[0].framework, "AoUForwarding")

    def test_marker_location_uses_yaml_path(self) -> None:
        items = [_req("req Pkg.AoU1@1", "AoU1")]
        entries = [{"aou_id": "Pkg.AoU1", "justification": "reason"}]
        markers = build_forwarded_markers(entries, items, "some/path/aou_forwarding.yaml")
        self.assertEqual(markers[0].location.filename, "some/path/aou_forwarding.yaml")

    def test_nonexistent_aou_id_raises(self) -> None:
        items = [_req("req Pkg.AoU1@1", "AoU1")]
        entries = [{"aou_id": "NonExistent.Foo", "justification": "reason"}]
        with self.assertRaises(SystemExit):
            build_forwarded_markers(entries, items, "aou_forwarding.yaml")

    def test_empty_entries_produces_no_markers(self) -> None:
        items = [_req("req Pkg.AoU1@1", "AoU1")]
        markers = build_forwarded_markers([], items, "aou_forwarding.yaml")
        self.assertEqual(markers, [])


if __name__ == "__main__":
    unittest.main()
