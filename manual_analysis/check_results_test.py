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

import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from manual_analysis import check_results


class CheckResultsTest(unittest.TestCase):
    def _write_results(self, path: Path, results: list[dict]) -> None:
        payload = {
            "analysis": "analysis.yaml",
            "results": results,
        }
        path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

    def test_accepts_positive_final_assertion(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            results_path = Path(tmpdir) / "results.json"
            self._write_results(
                results_path,
                [
                    {"type": "action", "description": "step", "result": "ok"},
                    {
                        "type": "assertion",
                        "description": "Any errors?",
                        "answer": "No",
                        "passed": True,
                    },
                ],
            )

            with mock.patch.dict(
                os.environ,
                {"MANUAL_ANALYSIS_RESULTS_FILE": str(results_path)},
                clear=False,
            ):
                check_results.main([])

    def test_rejects_failed_final_assertion(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            results_path = Path(tmpdir) / "results.json"
            self._write_results(
                results_path,
                [
                    {
                        "type": "assertion",
                        "description": "Any errors?",
                        "answer": "Yes",
                        "passed": False,
                    }
                ],
            )

            with mock.patch.dict(
                os.environ,
                {"MANUAL_ANALYSIS_RESULTS_FILE": str(results_path)},
                clear=False,
            ):
                with self.assertRaises(SystemExit) as cm:
                    check_results.main([])

            self.assertEqual(getattr(cm.exception, "code", None), 1)

    def test_rejects_partial_run_marker(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            results_path = Path(tmpdir) / "results.json"
            self._write_results(
                results_path,
                [
                    {"type": "action", "description": "step", "result": "ok"},
                    {
                        "type": "assertion",
                        "description": "Final assertion (partial run)",
                        "answer": "No",
                        "passed": False,
                        "partial_run": True,
                    },
                ],
            )

            with mock.patch.dict(
                os.environ,
                {"MANUAL_ANALYSIS_RESULTS_FILE": str(results_path)},
                clear=False,
            ):
                with self.assertRaises(SystemExit) as cm:
                    check_results.main([])

            self.assertEqual(getattr(cm.exception, "code", None), 1)

    def test_rejects_when_last_result_is_not_assertion(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            results_path = Path(tmpdir) / "results.json"
            self._write_results(
                results_path,
                [{"type": "action", "description": "step", "result": "ok"}],
            )

            with mock.patch.dict(
                os.environ,
                {"MANUAL_ANALYSIS_RESULTS_FILE": str(results_path)},
                clear=False,
            ):
                with self.assertRaises(SystemExit) as cm:
                    check_results.main([])

            self.assertEqual(getattr(cm.exception, "code", None), 1)


if __name__ == "__main__":
    unittest.main()
