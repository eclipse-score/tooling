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

import hashlib
import tempfile
import unittest
from pathlib import Path

import manual_analysis.update_lock as update_lock


class UpdateLockTest(unittest.TestCase):
    def test_write_lock_sorts_rows_by_display_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            lock_path = Path(tmpdir) / "result.lock"
            rows = [
                ("z/path", "hash-z"),
                ("a/path", "hash-a"),
            ]

            update_lock._write_lock(rows, lock_path)

            self.assertEqual(
                lock_path.read_text(encoding="utf-8"),
                "hash-a a/path\nhash-z z/path\n",
            )

    def test_main_writes_combined_file_and_rule_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            temp = Path(tmpdir)
            source_a = temp / "a.txt"
            source_b = temp / "b.txt"
            source_a.write_text("A", encoding="utf-8")
            source_b.write_text("B", encoding="utf-8")

            files_manifest = temp / "files.tsv"
            files_manifest.write_text(
                f"b/source\t{source_b}\na/source\t{source_a}\n",
                encoding="utf-8",
            )

            rules_manifest = temp / "rules.tsv"
            rules_manifest.write_text(
                "c/rule\tname=rule_c;version=1\n",
                encoding="utf-8",
            )

            output = temp / "manual.lock"
            update_lock.main(
                [
                    "--files-manifest",
                    str(files_manifest),
                    "--rules-manifest",
                    str(rules_manifest),
                    "--output",
                    str(output),
                ]
            )

            expected_file_a = hashlib.sha256(b"A").hexdigest()
            expected_file_b = hashlib.sha256(b"B").hexdigest()
            expected_rule = hashlib.sha256(b"name=rule_c;version=1").hexdigest()

            self.assertEqual(
                output.read_text(encoding="utf-8"),
                f"{expected_file_a} a/source\n"
                f"{expected_file_b} b/source\n"
                f"{expected_rule} c/rule\n",
            )

    def test_main_fails_when_files_manifest_is_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            temp = Path(tmpdir)
            rules_manifest = temp / "rules.tsv"
            rules_manifest.write_text("rule\tcanonical\n", encoding="utf-8")

            with self.assertRaises(SystemExit) as cm:
                update_lock.main(
                    [
                        "--files-manifest",
                        str(temp / "missing.tsv"),
                        "--rules-manifest",
                        str(rules_manifest),
                        "--output",
                        str(temp / "result.lock"),
                    ]
                )

            self.assertEqual(getattr(cm.exception, "code", None), 1)


if __name__ == "__main__":
    unittest.main()
