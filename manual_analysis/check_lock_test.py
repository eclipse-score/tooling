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

import tempfile
import unittest
from pathlib import Path

import manual_analysis.check_lock as check_lock


class CheckLockTest(unittest.TestCase):
    def test_evaluate_succeeds_when_locks_match(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            computed = Path(tmpdir) / "computed.lock"
            committed = Path(tmpdir) / "committed.lock"
            content = "abc path\n"
            computed.write_text(content, encoding="utf-8")
            committed.write_text(content, encoding="utf-8")

            is_ok, error = check_lock.evaluate_lock_files(computed, committed)
            self.assertTrue(is_ok)
            self.assertIsNone(error)

    def test_evaluate_fails_when_computed_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            computed = Path(tmpdir) / "missing.lock"
            committed = Path(tmpdir) / "committed.lock"
            committed.write_text("x\n", encoding="utf-8")

            is_ok, error = check_lock.evaluate_lock_files(computed, committed)
            self.assertFalse(is_ok)
            self.assertIn("computed lock file not found", error or "")

    def test_evaluate_fails_when_contents_differ(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            computed = Path(tmpdir) / "computed.lock"
            committed = Path(tmpdir) / "committed.lock"
            computed.write_text("a path\n", encoding="utf-8")
            committed.write_text("b path\n", encoding="utf-8")

            is_ok, error = check_lock.evaluate_lock_files(computed, committed)
            self.assertFalse(is_ok)
            self.assertEqual(error, "Manual analysis lock file is out of date.")


if __name__ == "__main__":
    unittest.main()
