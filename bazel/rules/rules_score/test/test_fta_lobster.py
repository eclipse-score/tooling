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
"""Tests for fta_lobster and fta_index."""

import json
import os
import sys
import tempfile
import unittest

from lobster.common.io import lobster_read, lobster_write
from lobster.common.items import Requirement, Tracing_Tag
from lobster.common.errors import Message_Handler
from lobster.common.location import File_Reference

import fta_lobster
from fta_index import Finding, FtaIndex, SourceRef


def _failure_mode(name: str, version: int = 1) -> Requirement:
    return Requirement(
        tag=Tracing_Tag("req", f"Lib.{name}", str(version)),
        location=File_Reference("fm.trlc", line=1),
        framework="TRLC",
        kind="FailureMode",
        name=f"Lib.{name}",
    )


def _index(findings: list[Finding], diagrams: dict[str, str] | None = None) -> FtaIndex:
    return FtaIndex(diagrams=diagrams or {}, failure_mode_trees={}, via={}, findings=findings)


class _TempDirTest(unittest.TestCase):
    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = tmp.name

    def path(self, name: str) -> str:
        return os.path.join(self.tmp, name)

    def write_lobster(self, name: str, items: list[Requirement]) -> str:
        path = self.path(name)
        with open(path, "w", encoding="utf-8") as fh:
            lobster_write(fh, Requirement, "test", items)
        return path

    def write_text(self, name: str, text: str) -> str:
        path = self.path(name)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text)
        return path


class KnownFailureModesTest(_TempDirTest):
    def test_names_of_all_files_are_sorted_and_unique(self) -> None:
        first = self.write_lobster("a.lobster", [_failure_mode("B"), _failure_mode("A")])
        second = self.write_lobster("b.lobster", [_failure_mode("B"), _failure_mode("C")])
        self.assertEqual(fta_lobster.known_failure_modes([first, second]), ["Lib.A", "Lib.B", "Lib.C"])

    def test_version_is_not_part_of_the_name(self) -> None:
        path = self.write_lobster("a.lobster", [_failure_mode("A", version=7)])
        self.assertEqual(fta_lobster.known_failure_modes([path]), ["Lib.A"])

    def test_no_files_know_no_failure_modes(self) -> None:
        self.assertEqual(fta_lobster.known_failure_modes([]), [])

    def test_unreadable_file_exits(self) -> None:
        with self.assertRaises(SystemExit) as ctx:
            fta_lobster.known_failure_modes([self.path("missing.lobster")])
        self.assertIn("missing.lobster", str(ctx.exception))

    def test_malformed_file_exits(self) -> None:
        path = self.write_text("bad.lobster", "not json")
        with self.assertRaises(SystemExit):
            fta_lobster.known_failure_modes([path])


class FindingItemsTest(unittest.TestCase):
    def test_item_is_tagged_by_diagram_line_and_kind(self) -> None:
        items = fta_lobster.finding_items(
            _index([Finding("UndevelopedEvent", "fta_a.puml", 7, "no cause")], {"fta_a.puml": "pkg/fta_a.puml"}),
            "Lib_fta",
        )
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0].tag.key(), "fta Lib_fta.fta_a_7_UndevelopedEvent")
        self.assertEqual(items[0].kind, "UndevelopedEvent")
        self.assertEqual(items[0].framework, "puml_fta")

    def test_item_carries_its_message_as_error(self) -> None:
        item = fta_lobster.finding_items(_index([Finding("UndevelopedEvent", "a.puml", 7, "no cause")]), "P")[0]
        self.assertEqual(item.messages, ["no cause"])
        self.assertTrue(item.has_error)

    def test_location_is_the_diagram_source_path_and_line(self) -> None:
        item = fta_lobster.finding_items(
            _index([Finding("UndevelopedEvent", "a.puml", 7, "m")], {"a.puml": "pkg/a.puml"}), "P"
        )[0]
        self.assertEqual(item.location.filename, "pkg/a.puml")
        self.assertEqual(item.location.line, 7)

    def test_diagram_without_source_path_falls_back_to_its_name(self) -> None:
        item = fta_lobster.finding_items(_index([Finding("UndevelopedEvent", "a.puml", 7, "m")]), "P")[0]
        self.assertEqual(item.location.filename, "a.puml")

    def test_finding_without_line_has_no_line(self) -> None:
        item = fta_lobster.finding_items(_index([Finding("TitleMismatch", "a.puml", 0, "m")]), "P")[0]
        self.assertIsNone(item.location.line)

    def test_name_has_only_identifier_characters(self) -> None:
        item = fta_lobster.finding_items(_index([Finding("Kind", "my-fta.v2.puml", 1, "m")]), "P")[0]
        self.assertEqual(item.name, "my_fta_v2_1_Kind")

    def test_equal_findings_get_distinct_tags(self) -> None:
        findings = [Finding("Kind", "a.puml", 1, "m")] * 3
        tags = [item.tag.key() for item in fta_lobster.finding_items(_index(findings), "P")]
        self.assertEqual(tags, ["fta P.a_1_Kind", "fta P.a_1_Kind_2", "fta P.a_1_Kind_3"])

    def test_no_findings_no_items(self) -> None:
        self.assertEqual(fta_lobster.finding_items(FtaIndex.empty(), "P"), [])


class MainTest(_TempDirTest):
    def run_main(self, *argv: str) -> None:
        saved = sys.argv
        try:
            sys.argv = ["fta_lobster", *argv]
            fta_lobster.main()
        finally:
            sys.argv = saved

    def test_known_failure_modes_writes_one_name_per_line(self) -> None:
        lobster = self.write_lobster("a.lobster", [_failure_mode("B"), _failure_mode("A")])
        output = self.path("known.txt")
        self.run_main("known-failure-modes", "--output", output, lobster)
        with open(output, encoding="utf-8") as fh:
            self.assertEqual(fh.read(), "Lib.A\nLib.B\n")

    def test_known_failure_modes_without_files_writes_an_empty_file(self) -> None:
        output = self.path("known.txt")
        self.run_main("known-failure-modes", "--output", output)
        with open(output, encoding="utf-8") as fh:
            self.assertEqual(fh.read(), "")

    def test_findings_are_written_as_lobster_items_that_read_back_with_their_message(self) -> None:
        index = self.write_text(
            "fta_index.json",
            json.dumps(
                {
                    "version": 1,
                    "diagrams": {"a.puml": "pkg/a.puml"},
                    "failure_mode_trees": {},
                    "via": {},
                    "findings": [{"kind": "UndevelopedEvent", "diagram": "a.puml", "line": 4, "message": "no cause"}],
                }
            ),
        )
        output = self.path("fta_findings.lobster")
        self.run_main("findings", "--index", index, "--package", "Lib_fta", "--output", output)

        items: dict[str, Requirement] = {}
        lobster_read(Message_Handler(), output, "findings", items)
        self.assertEqual(list(items), ["fta Lib_fta.a_4_UndevelopedEvent"])
        self.assertEqual(items["fta Lib_fta.a_4_UndevelopedEvent"].messages, ["no cause"])

    def test_findings_of_an_unsupported_index_version_exit(self) -> None:
        index = self.write_text("fta_index.json", json.dumps({"version": 2}))
        with self.assertRaises(SystemExit) as ctx:
            self.run_main("findings", "--index", index, "--package", "P", "--output", self.path("out.lobster"))
        self.assertIn("unsupported fta_index version", str(ctx.exception))


class FtaIndexTest(_TempDirTest):
    def load(self, data: object) -> FtaIndex:
        return FtaIndex.load(self.write_text("fta_index.json", json.dumps(data)))

    def test_all_sections_are_read(self) -> None:
        index = self.load(
            {
                "version": 1,
                "diagrams": {"a.puml": "pkg/a.puml"},
                "failure_mode_trees": {"Lib.FM": [{"diagram": "a.puml", "line": 1, "alias": "fm"}]},
                "via": {"RC": {"Lib.FM": [{"diagram": "a.puml", "line": 9, "alias": "T1"}]}},
                "findings": [{"kind": "K", "diagram": "a.puml", "line": 3, "message": "m"}],
            }
        )
        self.assertEqual(index.diagrams, {"a.puml": "pkg/a.puml"})
        self.assertEqual(index.failure_mode_trees["Lib.FM"], [SourceRef("a.puml", 1, "fm")])
        self.assertEqual(index.via["RC"]["Lib.FM"], [SourceRef("a.puml", 9, "T1")])
        self.assertEqual(index.findings, [Finding("K", "a.puml", 3, "m")])

    def test_other_version_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "unsupported fta_index version 2"):
            self.load({"version": 2})

    def test_missing_version_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "unsupported fta_index version None"):
            self.load({"findings": []})

    def test_non_object_is_rejected(self) -> None:
        with self.assertRaises(ValueError):
            self.load([])

    def test_empty_index_has_no_content(self) -> None:
        index = FtaIndex.empty()
        self.assertEqual((index.diagrams, index.failure_mode_trees, index.via, index.findings), ({}, {}, {}, []))


if __name__ == "__main__":
    unittest.main()
