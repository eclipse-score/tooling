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

import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import manual_analysis.check_lock as check_lock


class CheckLockTest(unittest.TestCase):
    def test_main_succeeds_when_locks_match(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            computed = Path(tmpdir) / "computed.lock"
            committed = Path(tmpdir) / "committed.lock"
            content = "abc path\n"
            computed.write_text(content, encoding="utf-8")
            committed.write_text(content, encoding="utf-8")

            check_lock.main(
                ["--computed", str(computed), "--committed", str(committed)]
            )

    def test_main_fails_when_computed_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            computed = Path(tmpdir) / "missing.lock"
            committed = Path(tmpdir) / "committed.lock"
            committed.write_text("x\n", encoding="utf-8")

            with self.assertRaises(SystemExit) as cm:
                check_lock.main(
                    ["--computed", str(computed), "--committed", str(committed)]
                )

            self.assertEqual(getattr(cm.exception, "code", None), 1)

    def test_main_fails_when_contents_differ(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            computed = Path(tmpdir) / "computed.lock"
            committed = Path(tmpdir) / "committed.lock"
            computed.write_text("a path\n", encoding="utf-8")
            committed.write_text("b path\n", encoding="utf-8")

            with self.assertRaises(SystemExit) as cm:
                check_lock.main(
                    ["--computed", str(computed), "--committed", str(committed)]
                )

            self.assertEqual(getattr(cm.exception, "code", None), 1)

    def test_main_uses_environment_defaults(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            computed = Path(tmpdir) / "computed.lock"
            committed = Path(tmpdir) / "committed.lock"
            content = "same lock\n"
            computed.write_text(content, encoding="utf-8")
            committed.write_text(content, encoding="utf-8")

            with mock.patch.dict(
                os.environ,
                {
                    "MANUAL_ANALYSIS_COMPUTED_LOCK": str(computed),
                    "MANUAL_ANALYSIS_COMMITTED_LOCK": str(committed),
                },
                clear=False,
            ):
                check_lock.main([])


if __name__ == "__main__":
    unittest.main()
