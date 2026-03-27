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
import tempfile
import unittest
from pathlib import Path

from manual_analysis import check_results


class CheckResultsTest(unittest.TestCase):
    def _write_results(self, path: Path, results: list[dict]) -> None:
        payload = {
            "analysis": "analysis.yaml",
            "results": results,
        }
        path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

    def test_evaluate_accepts_positive_final_assertion(self) -> None:
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

            is_ok, error = check_results.evaluate_results_file(results_path)
            self.assertTrue(is_ok)
            self.assertIsNone(error)

    def test_evaluate_rejects_failed_final_assertion(self) -> None:
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

            is_ok, error = check_results.evaluate_results_file(results_path)
            self.assertFalse(is_ok)
            self.assertEqual(
                error,
                "Final manual analysis assertion is not positive.",
            )

    def test_evaluate_rejects_partial_run_marker(self) -> None:
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

            is_ok, error = check_results.evaluate_results_file(results_path)
            self.assertFalse(is_ok)
            self.assertEqual(
                error,
                "Final manual analysis assertion is not positive.",
            )

    def test_evaluate_rejects_when_last_result_is_not_assertion(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            results_path = Path(tmpdir) / "results.json"
            self._write_results(
                results_path,
                [{"type": "action", "description": "step", "result": "ok"}],
            )

            is_ok, error = check_results.evaluate_results_file(results_path)
            self.assertFalse(is_ok)
            self.assertEqual(
                error,
                "Manual analysis does not end with an assertion.",
            )


    def test_evaluate_returns_error_on_unreadable_file(self) -> None:
        from unittest import mock

        with tempfile.TemporaryDirectory() as tmpdir:
            results_path = Path(tmpdir) / "results.json"
            results_path.write_text("{}", encoding="utf-8")
            with mock.patch.object(Path, "read_text", side_effect=OSError("permission denied")):
                is_ok, error = check_results.evaluate_results_file(results_path)
            self.assertFalse(is_ok)
            self.assertIn("Could not read results file", error)
            self.assertIn("permission denied", error)

    def test_evaluate_returns_error_on_invalid_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            results_path = Path(tmpdir) / "results.json"
            results_path.write_text("not json {{", encoding="utf-8")
            is_ok, error = check_results.evaluate_results_file(results_path)
        self.assertFalse(is_ok)
        self.assertIn("Results file is not valid JSON", error)


if __name__ == "__main__":
    unittest.main()
