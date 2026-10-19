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
"""Tests for merge_lobster_items."""

import os
import tempfile
import unittest

from lobster.common.io import lobster_write
from lobster.common.items import Requirement, Tracing_Tag
from lobster.common.location import File_Reference

from merge_lobster_items import merge_items


def _req(tag: str, text: str = "must hold", source: str = "aou.trlc") -> Requirement:
    namespace, rest = tag.split(" ", 1)
    return Requirement(
        tag=Tracing_Tag.from_text(namespace, rest),
        location=File_Reference(source, line=1),
        framework="TRLC",
        kind="AoU",
        name=rest.split("@")[0],
        text=text,
    )


class TestMergeItems(unittest.TestCase):
    """Tests for merge_items."""

    def _write(self, items: list[Requirement]) -> str:
        f = tempfile.NamedTemporaryFile(mode="w", suffix=".lobster", delete=False)
        self.addCleanup(os.unlink, f.name)
        lobster_write(f, Requirement, "test", items)
        f.close()
        return f.name

    def test_distinct_items_are_all_kept(self) -> None:
        merged = merge_items([self._write([_req("req A.X@1")]), self._write([_req("req B.Y@1")])])
        self.assertEqual([str(i.tag) for i in merged], ["req A.X@1", "req B.Y@1"])

    def test_same_item_via_two_files_is_kept_once(self) -> None:
        merged = merge_items([self._write([_req("req A.X@1")]), self._write([_req("req A.X@1")])])
        self.assertEqual([str(i.tag) for i in merged], ["req A.X@1"])

    def test_same_item_via_three_files_is_kept_once(self) -> None:
        paths = [self._write([_req("req A.X@1")]) for _ in range(3)]
        self.assertEqual(len(merge_items(paths)), 1)

    def test_location_is_ignored_and_first_file_wins(self) -> None:
        origin = self._write([_req("req A.X@1", source="origin.trlc")])
        copy = self._write([_req("req A.X@1", source="copy.trlc")])
        merged = merge_items([origin, copy])
        self.assertEqual(len(merged), 1)
        self.assertEqual(merged[0].location.filename, "origin.trlc")

    def test_different_text_is_a_conflict(self) -> None:
        paths = [self._write([_req("req A.X@1", text="one")]), self._write([_req("req A.X@1", text="two")])]
        with self.assertRaises(SystemExit) as ctx:
            merge_items(paths)
        self.assertIn("Conflicting definitions of 'req A.X'", str(ctx.exception))
        self.assertIn("text", str(ctx.exception))

    def test_report_state_is_not_part_of_the_definition(self) -> None:
        plain = _req("req A.X@1")
        resolved = _req("req A.X@1")
        resolved.messages.append("checked")
        resolved.ref_up.append(Tracing_Tag("req", "B.Y", "1"))
        merged = merge_items([self._write([plain]), self._write([resolved])])
        self.assertEqual(len(merged), 1)

    def test_different_references_are_a_conflict(self) -> None:
        first = _req("req A.X@1")
        second = _req("req A.X@1")
        second.add_tracing_target(Tracing_Tag("req", "B.Y", "1"))
        with self.assertRaises(SystemExit) as ctx:
            merge_items([self._write([first]), self._write([second])])
        self.assertIn("refs", str(ctx.exception))

    def test_different_version_is_a_conflict(self) -> None:
        paths = [self._write([_req("req A.X@1")]), self._write([_req("req A.X@2")])]
        with self.assertRaises(SystemExit) as ctx:
            merge_items(paths)
        self.assertIn("tag", str(ctx.exception))

    def test_unreadable_file_is_reported(self) -> None:
        with self.assertRaises(SystemExit) as ctx:
            merge_items(["/nonexistent/items.lobster"])
        self.assertIn("Failed to parse lobster file", str(ctx.exception))

    def test_no_files_gives_no_items(self) -> None:
        self.assertEqual(merge_items([]), [])


if __name__ == "__main__":
    unittest.main()
