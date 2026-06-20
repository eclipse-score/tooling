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

import json
import tempfile
import unittest

import yaml

from aou_forwarding_to_lobster import (
    create_lobster_output,
    filter_forwarded_aous,
    load_lobster_items,
    parse_forwarding_yaml,
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

    def _write_lobster(self, items: list) -> str:
        data = {
            "schema": "lobster-req-trace",
            "version": 3,
            "generator": "test",
            "data": items,
        }
        f = tempfile.NamedTemporaryFile(mode="w", suffix=".lobster", delete=False)
        json.dump(data, f)
        f.close()
        return f.name

    def test_loads_items(self) -> None:
        items = [
            {"tag": "req Pkg.AoU1", "name": "AoU1"},
            {"tag": "req Pkg.AoU2", "name": "AoU2"},
        ]
        path = self._write_lobster(items)
        loaded = load_lobster_items([path])
        self.assertEqual(len(loaded), 2)
        self.assertEqual(loaded[0]["tag"], "req Pkg.AoU1")

    def test_multiple_files(self) -> None:
        path1 = self._write_lobster([{"tag": "req A.B", "name": "B"}])
        path2 = self._write_lobster([{"tag": "req C.D", "name": "D"}])
        loaded = load_lobster_items([path1, path2])
        self.assertEqual(len(loaded), 2)

    def test_empty_data(self) -> None:
        path = self._write_lobster([])
        loaded = load_lobster_items([path])
        self.assertEqual(loaded, [])


class TestFilterForwardedAous(unittest.TestCase):
    """Tests for filter_forwarded_aous."""

    def test_filters_correctly(self) -> None:
        items = [
            {"tag": "req Pkg.AoU1", "name": "AoU1"},
            {"tag": "req Pkg.AoU2", "name": "AoU2"},
        ]
        entries = [{"aou_id": "Pkg.AoU1", "justification": "reason"}]
        filtered = filter_forwarded_aous(entries, items)
        self.assertEqual(len(filtered), 1)
        self.assertEqual(filtered[0]["tag"], "req Pkg.AoU1")

    def test_multiple_filters(self) -> None:
        items = [
            {"tag": "req A.B", "name": "B"},
            {"tag": "req C.D", "name": "D"},
            {"tag": "req E.F", "name": "F"},
        ]
        entries = [
            {"aou_id": "A.B", "justification": "r1"},
            {"aou_id": "E.F", "justification": "r2"},
        ]
        filtered = filter_forwarded_aous(entries, items)
        self.assertEqual(len(filtered), 2)

    def test_nonexistent_aou_id_raises(self) -> None:
        items = [{"tag": "req Pkg.AoU1", "name": "AoU1"}]
        entries = [{"aou_id": "NonExistent.Foo", "justification": "reason"}]
        with self.assertRaises(SystemExit):
            filter_forwarded_aous(entries, items)

    def test_versioned_tag_matches_base_id(self) -> None:
        """lobster-trlc generates versioned tags like 'req Pkg.Name@1'."""
        items = [
            {"tag": "req Pkg.AoU1@1", "name": "AoU1"},
            {"tag": "req Pkg.AoU2@3", "name": "AoU2"},
        ]
        entries = [{"aou_id": "Pkg.AoU1", "justification": "reason"}]
        filtered = filter_forwarded_aous(entries, items)
        self.assertEqual(len(filtered), 1)
        self.assertEqual(filtered[0]["tag"], "req Pkg.AoU1@1")

    def test_versioned_tag_matches_full_id(self) -> None:
        """Full versioned ID should also work."""
        items = [{"tag": "req Pkg.AoU1@2", "name": "AoU1"}]
        entries = [{"aou_id": "Pkg.AoU1@2", "justification": "reason"}]
        filtered = filter_forwarded_aous(entries, items)
        self.assertEqual(len(filtered), 1)


class TestCreateLobsterOutput(unittest.TestCase):
    """Tests for create_lobster_output."""

    def test_wraps_items(self) -> None:
        items = [{"tag": "req Foo.Bar", "name": "Bar"}]
        output = create_lobster_output(items)
        self.assertEqual(output["schema"], "lobster-req-trace")
        self.assertEqual(output["version"], 3)
        self.assertEqual(output["generator"], "aou_forwarding_to_lobster")
        self.assertEqual(output["data"], items)

    def test_empty_items(self) -> None:
        output = create_lobster_output([])
        self.assertEqual(output["data"], [])


if __name__ == "__main__":
    unittest.main()
