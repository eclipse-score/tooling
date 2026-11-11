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

import unittest
from types import SimpleNamespace

from lobster.common.items import Activity, Requirement, Tracing_Tag
from lobster.common.location import File_Reference
from lobster_rst_report._helpers import RstUtils
from lobster_rst_report._renderers import ItemCardBuilder

_LIST_TABLE = """.. list-table::
   :header-rows: 1

   * - A
     - B
   * - 1
     - 2"""


def _requirement(framework: str, text: str) -> Requirement:
    return Requirement(
        tag=Tracing_Tag("req", "pkg.Name"),
        location=File_Reference("a.trlc", 1),
        framework=framework,
        kind="Requirement",
        name="pkg.Name",
        text=text,
    )


def _card(item) -> str:
    return "\n".join(ItemCardBuilder(item, SimpleNamespace(items={})).build())


class DescriptionTest(unittest.TestCase):
    def test_trlc_text_is_emitted_as_authored_rst(self):
        item = _requirement("TRLC", _LIST_TABLE)
        self.assertEqual(RstUtils.description(item), _LIST_TABLE.splitlines())

    def test_trlc_list_table_survives_in_card(self):
        card = _card(_requirement("TRLC", _LIST_TABLE))
        self.assertIn("      * - A", card)
        self.assertNotIn("\\*", card)

    def test_non_trlc_requirement_text_is_plain(self):
        item = _requirement("codebeamer", "- item_\n.. note:: x\n\n  *args")
        self.assertEqual(
            RstUtils.description(item),
            ["| - item\\_", "| .. note:: x", "|", "|   \\*args"],
        )

    def test_activity_text_is_plain(self):
        item = Activity(
            tag=Tracing_Tag("test", "t"),
            location=File_Reference("t.cc", 1),
            framework="GoogleTest",
            kind="test",
            text=":ref:`x` and a_b",
        )
        self.assertEqual(RstUtils.description(item), ["| :ref:\\`x\\` and a\\_b"])


class UnresolvedReferenceTest(unittest.TestCase):
    def test_unresolved_key_uses_literal_role(self):
        item = _requirement("TRLC", "x")
        item.ref_down = [Tracing_Tag("req", "A_B")]
        self.assertIn(":literal:`req A\\_B` (unresolved)", _card(item))


class HeadingTest(unittest.TestCase):
    def test_heading_text_is_escaped_and_underline_matches(self):
        self.assertEqual(RstUtils.heading("my_level_", "-"), ["my\\_level\\_", "-" * 11])

    def test_overline(self):
        self.assertEqual(RstUtils.heading("T", "#", overline=True), ["#", "T", "#"])


if __name__ == "__main__":
    unittest.main()
