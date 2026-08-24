#!/usr/bin/env python3
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
"""Unit tests for the markdown coverage summary."""

import json
import tempfile
import unittest
from pathlib import Path

from coverage.coverage_summary import (
    directory_key,
    load_justification_summary,
    parse_lcov,
    percent,
    progress_bar,
    render_markdown,
    rollup_by_directory,
)

LCOV_TWO_FILES = (
    "SF:src/foo/a.cpp\n"
    "DA:1,1\nDA:2,0\n"
    "BRF:4\nBRH:3\n"
    "LF:2\nLH:1\n"
    "end_of_record\n"
    "SF:rust/main.rs\n"
    "DA:1,0\n"
    "LF:3\nLH:0\n"
    "end_of_record\n"
)


def _write(tmp: str, name: str, content: str) -> Path:
    p = Path(tmp) / name
    p.write_text(content, encoding="utf-8")
    return p


class ParseLcovTest(unittest.TestCase):
    def test_missing_file_returns_none(self):
        self.assertIsNone(parse_lcov(Path("/nonexistent/lcov.dat")))

    def test_empty_file_returns_empty_list(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(parse_lcov(_write(tmp, "e.dat", "")), [])

    def test_line_and_branch_counters(self):
        with tempfile.TemporaryDirectory() as tmp:
            files = parse_lcov(_write(tmp, "l.dat", LCOV_TWO_FILES))
        self.assertEqual(len(files), 2)
        a, main_rs = files
        self.assertEqual((a.lines_hit, a.lines_found), (1, 2))
        self.assertEqual((a.branches_hit, a.branches_found), (3, 4))
        self.assertEqual((main_rs.lines_hit, main_rs.lines_found), (0, 3))

    def test_lf_without_brf_yields_no_branch_data(self):
        with tempfile.TemporaryDirectory() as tmp:
            files = parse_lcov(_write(tmp, "l.dat", "SF:a.cpp\nLF:5\nLH:2\nend_of_record\n"))
        self.assertIsNone(files[0].branches_found)

    def test_brda_fallback_when_no_brf(self):
        lcov = "SF:a.cpp\nBRDA:1,0,0,3\nBRDA:1,0,1,-\nBRDA:2,0,0,0\nLF:2\nLH:2\nend_of_record\n"
        with tempfile.TemporaryDirectory() as tmp:
            files = parse_lcov(_write(tmp, "l.dat", lcov))
        self.assertEqual((files[0].branches_hit, files[0].branches_found), (1, 3))

    def test_record_without_end_of_record_is_flushed(self):
        with tempfile.TemporaryDirectory() as tmp:
            files = parse_lcov(_write(tmp, "l.dat", "SF:a.cpp\nLF:1\nLH:1\n"))
        self.assertEqual(len(files), 1)

    def test_non_utf8_bytes_do_not_crash(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "l.dat"
            p.write_bytes(b"SF:src/\xff\xfe.cpp\nLF:1\nLH:0\nend_of_record\n")
            files = parse_lcov(p)
        self.assertEqual(len(files), 1)
        self.assertEqual(files[0].lines_found, 1)


class MathHelpersTest(unittest.TestCase):
    def test_percent_zero_denominator_is_none(self):
        self.assertIsNone(percent(0, 0))
        self.assertIsNone(percent(5, 0))

    def test_progress_bar_bounds(self):
        self.assertEqual(progress_bar(0.0), "`" + "░" * 10 + "`")
        self.assertEqual(progress_bar(100.0), "`" + "█" * 10 + "`")
        self.assertEqual(progress_bar(None), "—")

    def test_directory_key_grouping(self):
        self.assertEqual(directory_key("a.cpp"), "(root)")
        self.assertEqual(directory_key("src/a.cpp"), "src")
        self.assertEqual(directory_key("src/foo/a.cpp"), "src/foo")
        self.assertEqual(directory_key("src/foo/bar/a.cpp"), "src/foo")


class RollupTest(unittest.TestCase):
    def test_worst_directory_first(self):
        with tempfile.TemporaryDirectory() as tmp:
            files = parse_lcov(_write(tmp, "l.dat", LCOV_TWO_FILES))
        rows = rollup_by_directory(files)
        self.assertEqual(rows[0]["directory"], "rust")
        self.assertEqual(rows[0]["pct"], 0.0)
        self.assertEqual(rows[1]["directory"], "src/foo")


class RenderTest(unittest.TestCase):
    def _render(self, justification=None):
        with tempfile.TemporaryDirectory() as tmp:
            files = parse_lcov(_write(tmp, "l.dat", LCOV_TWO_FILES))
        return render_markdown(files, justification)

    def test_empty_input_renders_note(self):
        md = render_markdown([], None)
        self.assertIn("No coverage records", md)

    def test_overall_table_and_zero_section(self):
        md = self._render()
        self.assertIn("| Lines | 1 | 5 | 20.00% |", md)
        self.assertIn("| Branches | 3 | 4 | 75.00% |", md)
        self.assertIn("Files at exact 0% (1)", md)
        self.assertIn("`rust/main.rs` (3 lines)", md)
        self.assertIn("█", md)  # progress bars present
        self.assertIn("<details>", md)

    def test_justification_section(self):
        justification = {
            "raw_line_coverage_pct": 20.0,
            "effective_line_coverage_pct": 40.0,
            "raw_branch_coverage_pct": 75.0,
            "effective_branch_coverage_pct": 75.0,
            "justified_lines": 1,
            "justified_branches": 0,
            "stale_justifications": 0,
            "applied_justification_count": 1,
        }
        md = self._render(justification)
        self.assertIn("Raw vs effective", md)
        self.assertIn("| Line coverage | 20.0% | 40.0% |", md)
        self.assertIn("1 justification entries applied", md)

    def test_branch_dash_when_no_branch_data(self):
        with tempfile.TemporaryDirectory() as tmp:
            files = parse_lcov(_write(tmp, "l.dat", "SF:a.cpp\nLF:2\nLH:1\nend_of_record\n"))
        md = render_markdown(files, None)
        self.assertIn("| Branches | — | — | — | — |", md)


class JustificationReportTest(unittest.TestCase):
    def test_loads_summary_and_counts_applied(self):
        report = {
            "version": 1,
            "summary": {"raw_line_coverage_pct": 50.0, "justified_lines": 2},
            "applied_justifications": [{"id": "a"}, {"id": "b"}],
        }
        with tempfile.TemporaryDirectory() as tmp:
            p = _write(tmp, "report.json", json.dumps(report))
            summary = load_justification_summary(p)
        self.assertEqual(summary["applied_justification_count"], 2)
        self.assertEqual(summary["justified_lines"], 2)

    def test_malformed_json_returns_none(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = _write(tmp, "report.json", "{not json")
            self.assertIsNone(load_justification_summary(p))


if __name__ == "__main__":
    unittest.main()
