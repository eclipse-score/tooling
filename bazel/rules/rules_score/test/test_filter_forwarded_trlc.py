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
"""Tests for filter_forwarded_trlc."""

import unittest

from filter_forwarded_trlc import check_all_entries_matched, extract_records, filter_trlc_source, parse_package

_SAMPLE = """\
/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 ********************************************************************************/
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

ScoreReq.AoU AoU3 {
    description = "Third AoU."
    safety      = ScoreReq.Asil.B
    version     = 1
}
"""

_NO_PACKAGE = """\
import ScoreReq

ScoreReq.AoU AoU1 {
    description = "First AoU."
}
"""


class TestParsePackage(unittest.TestCase):
    """Tests for parse_package."""

    def test_extracts_package_name(self) -> None:
        self.assertEqual(parse_package(_SAMPLE, "sample.trlc"), "MyAoUs")

    def test_missing_package_raises(self) -> None:
        with self.assertRaises(SystemExit):
            parse_package(_NO_PACKAGE, "no_package.trlc")


class TestExtractRecords(unittest.TestCase):
    """Tests for extract_records."""

    def test_extracts_all_records_in_order(self) -> None:
        records = extract_records(_SAMPLE)
        names = [name for _, name, _, _, _ in records]
        self.assertEqual(names, ["AoU1", "AoU2", "AoU3"])

    def test_record_text_spans_full_block(self) -> None:
        records = extract_records(_SAMPLE)
        _, _, text, _, _ = records[0]
        self.assertTrue(text.startswith("ScoreReq.AoU AoU1 {"))
        self.assertTrue(text.rstrip().endswith("}"))
        self.assertIn('description = "First AoU."', text)
        self.assertNotIn("AoU2", text)

    def test_no_records_returns_empty_list(self) -> None:
        header_only = "package P\n\nimport ScoreReq\n"
        self.assertEqual(extract_records(header_only), [])

    def test_unterminated_record_raises(self) -> None:
        broken = 'package P\n\nScoreReq.AoU AoU1 {\n    description = "x"\n'
        with self.assertRaises(SystemExit):
            extract_records(broken)


class TestFilterTrlcSource(unittest.TestCase):
    """Tests for filter_trlc_source."""

    def test_keeps_only_matched_records(self) -> None:
        entries = [{"aou_id": "MyAoUs.AoU2", "justification": "reason"}]
        result, matched = filter_trlc_source(_SAMPLE, entries)
        self.assertIn("AoU2", result)
        self.assertNotIn("AoU1", result)
        self.assertNotIn("AoU3", result)
        self.assertEqual(matched, {"MyAoUs.AoU2"})

    def test_matched_records_are_retyped_to_forwarded_aou(self) -> None:
        entries = [{"aou_id": "MyAoUs.AoU2", "justification": "reason"}]
        result, _ = filter_trlc_source(_SAMPLE, entries)
        self.assertIn("ScoreReq.ReceivedAoU AoU2 {", result)
        self.assertNotIn("ScoreReq.AoU AoU2", result)

    def test_matched_records_get_justification_injected(self) -> None:
        entries = [{"aou_id": "MyAoUs.AoU2", "justification": "because reasons"}]
        result, _ = filter_trlc_source(_SAMPLE, entries)
        self.assertIn('justification = "because reasons"', result)

    def test_justification_is_escaped(self) -> None:
        entries = [{"aou_id": "MyAoUs.AoU1", "justification": 'contains "quotes" and \\backslash'}]
        result, _ = filter_trlc_source(_SAMPLE, entries)
        self.assertIn('justification = "contains \\"quotes\\" and \\\\backslash"', result)

    def test_original_fields_preserved_alongside_justification(self) -> None:
        entries = [{"aou_id": "MyAoUs.AoU1", "justification": "reason"}]
        result, _ = filter_trlc_source(_SAMPLE, entries)
        self.assertIn('description = "First AoU."', result)
        self.assertIn("safety      = ScoreReq.Asil.B", result)
        self.assertIn("version     = 1", result)

    def test_multiple_matches_preserve_original_order(self) -> None:
        entries = [
            {"aou_id": "MyAoUs.AoU3", "justification": "r1"},
            {"aou_id": "MyAoUs.AoU1", "justification": "r2"},
        ]
        result, matched = filter_trlc_source(_SAMPLE, entries)
        self.assertLess(result.index("AoU1"), result.index("AoU3"))
        self.assertEqual(matched, {"MyAoUs.AoU1", "MyAoUs.AoU3"})

    def test_header_preserved_even_with_zero_matches(self) -> None:
        result, matched = filter_trlc_source(_SAMPLE, [])
        self.assertIn("package MyAoUs", result)
        self.assertIn("import ScoreReq", result)
        self.assertNotIn("AoU1", result)
        self.assertNotIn("AoU2", result)
        self.assertNotIn("AoU3", result)
        self.assertEqual(matched, set())

    def test_output_is_syntactically_plausible_with_zero_matches(self) -> None:
        """Header-only output must still contain the package statement so
        a downstream TRLC parse of this (now-empty) file doesn't choke on a
        missing package declaration."""
        result, _ = filter_trlc_source(_SAMPLE, [])
        self.assertTrue(result.strip().endswith("import ScoreReq"))

    def test_non_matching_ids_in_other_packages_are_ignored(self) -> None:
        """An aou_id belonging to a different package must not match here,
        since matching is scoped to Package.RecordName."""
        entries = [{"aou_id": "OtherPkg.AoU1", "justification": "reason"}]
        result, matched = filter_trlc_source(_SAMPLE, entries)
        self.assertNotIn("AoU1 {", result)
        self.assertEqual(matched, set())

    def test_versioned_aou_id_matches_ignoring_version(self) -> None:
        """TRLC record identity has no @version; the forwarding YAML's
        @version suffix (if any) must be stripped before matching."""
        entries = [{"aou_id": "MyAoUs.AoU1@1", "justification": "reason"}]
        result, matched = filter_trlc_source(_SAMPLE, entries)
        self.assertIn("AoU1", result)
        self.assertEqual(matched, {"MyAoUs.AoU1"})

    def test_all_records_kept_when_all_selected(self) -> None:
        entries = [
            {"aou_id": "MyAoUs.AoU1", "justification": "r1"},
            {"aou_id": "MyAoUs.AoU2", "justification": "r2"},
            {"aou_id": "MyAoUs.AoU3", "justification": "r3"},
        ]
        result, matched = filter_trlc_source(_SAMPLE, entries)
        for name in ("AoU1", "AoU2", "AoU3"):
            self.assertIn(name, result)
        self.assertEqual(matched, {"MyAoUs.AoU1", "MyAoUs.AoU2", "MyAoUs.AoU3"})

    def test_multi_hop_forwarded_aou_is_also_retyped(self) -> None:
        """A record that is already ReceivedAoU (a second forwarding hop)
        must still be retyped to ReceivedAoU (a no-op type-wise) and get a
        fresh justification for this hop."""
        source = (
            "package Mid\n\nimport ScoreReq\n\n"
            'ScoreReq.ReceivedAoU Received1 {\n    justification = "hop 1"\n    version = 1\n}\n'
        )
        entries = [{"aou_id": "Mid.Received1", "justification": "hop 2"}]
        result, matched = filter_trlc_source(source, entries)
        self.assertIn("ScoreReq.ReceivedAoU Received1 {", result)
        self.assertIn('justification = "hop 2"', result)
        self.assertEqual(matched, {"Mid.Received1"})

    def test_multi_hop_forwarding_does_not_duplicate_justification_field(self) -> None:
        """Retyping an already-ReceivedAoU record must replace, not
        duplicate, the justification field -- a duplicate assignment of the
        same component is a TRLC error."""
        source = (
            "package Mid\n\nimport ScoreReq\n\n"
            'ScoreReq.ReceivedAoU Received1 {\n    justification = "hop 1"\n    version = 1\n}\n'
        )
        entries = [{"aou_id": "Mid.Received1", "justification": "hop 2"}]
        result, _ = filter_trlc_source(source, entries)
        self.assertEqual(result.count("justification ="), 1)
        self.assertNotIn("hop 1", result)


class TestCheckAllEntriesMatched(unittest.TestCase):
    """Tests for check_all_entries_matched."""

    def test_no_error_when_all_matched(self) -> None:
        check_all_entries_matched({"Pkg.A", "Pkg.B"}, {"Pkg.A", "Pkg.B", "Pkg.C"})

    def test_raises_when_entry_unmatched(self) -> None:
        with self.assertRaises(SystemExit):
            check_all_entries_matched({"Pkg.A", "Pkg.Typo"}, {"Pkg.A"})

    def test_error_message_lists_unmatched_and_available(self) -> None:
        with self.assertRaises(SystemExit) as ctx:
            check_all_entries_matched({"Pkg.Typo"}, {"Pkg.A", "Pkg.B"})
        message = str(ctx.exception)
        self.assertIn("Pkg.Typo", message)
        self.assertIn("Pkg.A", message)
        self.assertIn("Pkg.B", message)

    def test_error_message_handles_no_available_ids(self) -> None:
        with self.assertRaises(SystemExit) as ctx:
            check_all_entries_matched({"Pkg.Typo"}, set())
        self.assertIn("(none)", str(ctx.exception))


if __name__ == "__main__":
    unittest.main()
