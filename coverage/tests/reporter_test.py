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
"""Unit tests for the final coverage reporter."""

import tempfile
import unittest
import zipfile
from pathlib import Path

from coverage.reporter import (
    _filter_lcov,
    _make_html_paths_relative,
    _make_lcov_paths_relative,
    _read_ar_members,
    expand_rlib_archives,
    write_empty_output,
)


def _ar_header(name: str, size: int) -> bytes:
    """Build a 60-byte Unix ar member header."""
    return (f"{name:<16}{'0':<12}{'0':<6}{'0':<6}{'100644':<8}{size:<10}`\n").encode()


def _make_archive(members) -> bytes:
    """Build a Unix ar archive from (name, data) tuples."""
    blob = b"!<arch>\n"
    for name, data in members:
        blob += _ar_header(name, len(data)) + data
        if len(data) % 2 == 1:
            blob += b"\n"
    return blob


class ReadArMembersTest(unittest.TestCase):
    def test_non_archive_returns_empty(self):
        with tempfile.NamedTemporaryFile(suffix=".a") as f:
            f.write(b"\x7fELF not an archive")
            f.flush()
            self.assertEqual(_read_ar_members(f.name), [])

    def test_members_are_listed_with_sizes(self):
        blob = _make_archive([("lib.rmeta/", b"META"), ("foo.o/", b"OBJDATA")])
        with tempfile.NamedTemporaryFile(suffix=".a") as f:
            f.write(blob)
            f.flush()
            members = _read_ar_members(f.name)
        self.assertEqual([(m[0], m[2]) for m in members], [("lib.rmeta", 4), ("foo.o", 7)])

    def test_gnu_long_name_table_is_resolved(self):
        longnames = b"a_very_long_object_file_name.o/\n"
        blob = _make_archive([("//", longnames), ("/0", b"LONGOBJ")])
        with tempfile.NamedTemporaryFile(suffix=".a") as f:
            f.write(blob)
            f.flush()
            members = _read_ar_members(f.name)
        self.assertEqual([m[0] for m in members], ["a_very_long_object_file_name.o"])


class ExpandRlibArchivesTest(unittest.TestCase):
    def test_rlib_is_expanded_to_object_members(self):
        """Archives with a lib.rmeta member are replaced by their .o members."""
        blob = _make_archive([("lib.rmeta/", b"META"), ("crate.o/", b"OBJ1"), ("notes.txt/", b"TXT")])
        with tempfile.TemporaryDirectory() as tmp:
            rlib = Path(tmp) / "libcrate.a"
            rlib.write_bytes(blob)
            workdir = Path(tmp) / "extracted"
            result = expand_rlib_archives([str(rlib)], workdir)
            self.assertEqual(len(result), 1)
            self.assertTrue(result[0].endswith(".o"))
            self.assertEqual(Path(result[0]).read_bytes(), b"OBJ1")

    def test_plain_cc_archive_passes_through(self):
        blob = _make_archive([("mylib.o/", b"OBJ1")])
        with tempfile.TemporaryDirectory() as tmp:
            archive = Path(tmp) / "libcc.a"
            archive.write_bytes(blob)
            result = expand_rlib_archives([str(archive)], Path(tmp) / "x")
            self.assertEqual(result, [str(archive)])

    def test_executable_passes_through(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / "my_tool"
            binary.write_bytes(b"\x7fELF" + b"\x00" * 12)
            result = expand_rlib_archives([str(binary)], Path(tmp) / "x")
            self.assertEqual(result, [str(binary)])


class FilterLcovTest(unittest.TestCase):
    LCOV = "SF:src/foo.cpp\nDA:1,1\nLF:1\nLH:1\nend_of_record\nSF:src/bar.cpp\nDA:1,0\nLF:1\nLH:0\nend_of_record\n"

    def test_only_target_records_survive(self):
        result = _filter_lcov(self.LCOV, {"src/bar.cpp"})
        self.assertIn("SF:src/bar.cpp", result)
        self.assertNotIn("SF:src/foo.cpp", result)

    def test_suffix_matching(self):
        result = _filter_lcov("SF:/abs/prefix/src/foo.cpp\nend_of_record\n", {"src/foo.cpp"})
        self.assertIn("SF:/abs/prefix/src/foo.cpp", result)


class MakeLcovPathsRelativeTest(unittest.TestCase):
    def test_workspace_paths_become_relative(self):
        lcov = "SF:/ws/root/src/foo.cpp\nDA:1,1\nend_of_record\n"
        result = _make_lcov_paths_relative(lcov, "/ws/root/")
        self.assertEqual(result, "SF:src/foo.cpp\nDA:1,1\nend_of_record\n")

    def test_workspace_root_without_trailing_slash(self):
        lcov = "SF:/ws/root/src/foo.cpp\nend_of_record\n"
        result = _make_lcov_paths_relative(lcov, "/ws/root")
        self.assertEqual(result, "SF:src/foo.cpp\nend_of_record\n")

    def test_external_paths_are_unchanged(self):
        lcov = "SF:/other/place/dep.cpp\nend_of_record\n"
        self.assertEqual(_make_lcov_paths_relative(lcov, "/ws/root/"), lcov)

    def test_non_sf_lines_are_preserved(self):
        lcov = "TN:test\nSF:/ws/root/a.cpp\nDA:5,0\nLF:1\nLH:0\nend_of_record\n"
        result = _make_lcov_paths_relative(lcov, "/ws/root/")
        self.assertIn("TN:test\n", result)
        self.assertIn("DA:5,0\n", result)


class MakeHtmlPathsRelativeTest(unittest.TestCase):
    def test_source_title_is_rewritten_and_hrefs_untouched(self):
        html = (
            "<div class='source-name-title'><pre>/ws/root/src/foo.cpp</pre></div>"
            "<a href='coverage/ws/root/src/foo.cpp.html'>link</a>"
        )
        with tempfile.TemporaryDirectory() as tmp:
            page = Path(tmp) / "page.html"
            page.write_text(html)
            _make_html_paths_relative(Path(tmp), "/ws/root/")
            result = page.read_text()
        self.assertIn("<pre>src/foo.cpp</pre>", result)
        # The href embeds the path components without a leading slash and must
        # never be rewritten.
        self.assertIn("href='coverage/ws/root/src/foo.cpp.html'", result)

    def test_missing_dir_is_a_noop(self):
        _make_html_paths_relative(Path("/nonexistent/html_dir"), "/ws/root/")


class WriteEmptyOutputTest(unittest.TestCase):
    def test_produces_valid_empty_zip(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out.zip"
            write_empty_output(out)
            self.assertTrue(zipfile.is_zipfile(out))
            with zipfile.ZipFile(out) as zf:
                self.assertEqual(zf.namelist(), [])


if __name__ == "__main__":
    unittest.main()
