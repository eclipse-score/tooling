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
"""Tests for dedupe_aou_trlc."""

import unittest

from dedupe_aou_trlc import dedupe_trlc_sources

_ORIGINAL = """\
package MyAoUs

import ScoreReq

ScoreReq.AoU AoU1 {
    description = "First AoU."
    safety      = ScoreReq.Asil.B
    version     = 1
}

ScoreReq.AoU AoU2 {
    description = "Second AoU."
    safety      = ScoreReq.Asil.B
    version     = 1
}
"""

_FORWARDED_COPY_OF_AOU1 = """\
package MyAoUs

import ScoreReq

ScoreReq.ReceivedAoU AoU1 {
    justification = "forwarded because reasons"
    description = "First AoU."
    safety      = ScoreReq.Asil.B
    version     = 1
}
"""

_UNRELATED = """\
package Other

import ScoreReq

ScoreReq.CompReq SomeReq {
    description = "Not an AoU at all."
    safety = ScoreReq.Asil.B
    derived_from = [MyAoUs.AoU2@1]
    version = 1
}
"""


class TestDedupeTrlcSources(unittest.TestCase):
    """Tests for dedupe_trlc_sources."""

    def test_no_duplicates_returns_sources_unchanged(self) -> None:
        sources = [("a.trlc", _ORIGINAL), ("b.trlc", _UNRELATED)]
        filtered, dropped = dedupe_trlc_sources(sources)
        self.assertEqual(filtered, [_ORIGINAL, _UNRELATED])
        self.assertEqual(dropped, {})

    def test_original_aou_wins_over_forwarded_copy(self) -> None:
        sources = [("a.trlc", _ORIGINAL), ("b.trlc", _FORWARDED_COPY_OF_AOU1)]
        filtered, dropped = dedupe_trlc_sources(sources)
        # a.trlc (the original) is untouched.
        self.assertEqual(filtered[0], _ORIGINAL)
        # b.trlc loses its ReceivedAoU copy of AoU1 but keeps its header.
        self.assertNotIn("AoU1", filtered[1])
        self.assertIn("package MyAoUs", filtered[1])
        self.assertEqual(dropped, {"MyAoUs.AoU1": "a.trlc"})

    def test_original_wins_regardless_of_input_order(self) -> None:
        sources = [("b.trlc", _FORWARDED_COPY_OF_AOU1), ("a.trlc", _ORIGINAL)]
        filtered, dropped = dedupe_trlc_sources(sources)
        self.assertNotIn("AoU1", filtered[0])
        self.assertEqual(filtered[1], _ORIGINAL)
        self.assertEqual(dropped, {"MyAoUs.AoU1": "a.trlc"})

    def test_two_forwarded_copies_pick_lexicographically_first_path(self) -> None:
        copy_a = _FORWARDED_COPY_OF_AOU1
        copy_b = _FORWARDED_COPY_OF_AOU1.replace("forwarded because reasons", "different reason")
        sources = [("z_hop.trlc", copy_a), ("a_hop.trlc", copy_b)]
        filtered, dropped = dedupe_trlc_sources(sources)
        self.assertNotIn("AoU1", filtered[0])
        self.assertIn("AoU1", filtered[1])
        self.assertEqual(dropped, {"MyAoUs.AoU1": "a_hop.trlc"})

    def test_non_aou_records_are_never_touched(self) -> None:
        """A colliding non-AoU/ReceivedAoU record type must be left alone --
        that is a genuine authoring error TRLC's own check should still
        catch, not something this tool silently resolves."""
        duplicate_comp_req = _UNRELATED
        sources = [("a.trlc", _UNRELATED), ("b.trlc", duplicate_comp_req)]
        filtered, dropped = dedupe_trlc_sources(sources)
        self.assertEqual(filtered, [_UNRELATED, duplicate_comp_req])
        self.assertEqual(dropped, {})

    def test_unrelated_records_in_a_deduped_file_are_preserved(self) -> None:
        mixed = (
            _FORWARDED_COPY_OF_AOU1
            + "\n"
            + _UNRELATED.replace("package Other", "package MyAoUs").replace("MyAoUs.AoU2@1", "AoU2@1")
        )
        sources = [("a.trlc", _ORIGINAL), ("b.trlc", mixed)]
        filtered, _dropped = dedupe_trlc_sources(sources)
        self.assertNotIn("ReceivedAoU AoU1", filtered[1])
        self.assertIn("SomeReq", filtered[1])

    def test_only_actual_duplicates_are_reported(self) -> None:
        sources = [("a.trlc", _ORIGINAL), ("b.trlc", _FORWARDED_COPY_OF_AOU1)]
        _filtered, dropped = dedupe_trlc_sources(sources)
        self.assertEqual(list(dropped.keys()), ["MyAoUs.AoU1"])
        self.assertNotIn("MyAoUs.AoU2", dropped)


if __name__ == "__main__":
    unittest.main()
