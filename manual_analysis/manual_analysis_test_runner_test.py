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

from manual_analysis import manual_analysis_test_runner


class ManualAnalysisTestRunnerTest(unittest.TestCase):
	def _write_file(self, path: Path, content: str) -> None:
		path.write_text(content, encoding="utf-8")

	def _write_results(self, path: Path, passed: bool) -> None:
		payload = {
			"analysis": "analysis.yaml",
			"results": [
				{
					"type": "assertion",
					"description": "Any errors?",
					"answer": "No" if passed else "Yes",
					"passed": passed,
				}
			],
		}
		path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

	def test_writes_fail_lobster_when_checks_fail(self) -> None:
		with tempfile.TemporaryDirectory() as tmpdir:
			tmp = Path(tmpdir)
			computed = tmp / "computed.txt"
			committed = tmp / "committed.txt"
			results = tmp / "results.json"
			analysis = tmp / "analysis.yaml"
			lobster = tmp / "out.lobster"

			self._write_file(computed, "hash-a\n")
			self._write_file(committed, "hash-b\n")
			self._write_results(results, passed=False)
			self._write_file(
				analysis,
				"requirements:\n  - REQ-1\nsteps:\n  - assertion:\n"
				"    description: done?\n    positive: Yes\n    negative: No\n",
			)

			with mock.patch.dict(
				os.environ,
				{
					"MANUAL_ANALYSIS_COMPUTED_LOCK": str(computed),
					"MANUAL_ANALYSIS_COMMITTED_LOCK": str(committed),
					"MANUAL_ANALYSIS_RESULTS_FILE": str(results),
					"MANUAL_ANALYSIS_YAML": str(analysis),
					"MANUAL_ANALYSIS_LABEL": "//manual_analysis/example:manual_analysis",
					"MANUAL_ANALYSIS_LOBSTER_OUTPUT": str(lobster),
				},
				clear=False,
			):
				manual_analysis_test_runner.main(["--allow-check-failures"])

			payload = json.loads(lobster.read_text(encoding="utf-8"))
			self.assertEqual(payload["schema"], "lobster-act-trace")
			self.assertEqual(payload["data"][0]["status"], "fail")

	def test_requirements_parse_error_exits_without_artifact(self) -> None:
		with tempfile.TemporaryDirectory() as tmpdir:
			tmp = Path(tmpdir)
			computed = tmp / "computed.txt"
			committed = tmp / "committed.txt"
			results = tmp / "results.json"
			analysis = tmp / "analysis.yaml"
			lobster = tmp / "out.lobster"

			self._write_file(computed, "hash\n")
			self._write_file(committed, "hash\n")
			self._write_results(results, passed=True)
			self._write_file(analysis, "requirements: []\nsteps: []\n")

			with mock.patch.dict(
				os.environ,
				{
					"MANUAL_ANALYSIS_COMPUTED_LOCK": str(computed),
					"MANUAL_ANALYSIS_COMMITTED_LOCK": str(committed),
					"MANUAL_ANALYSIS_RESULTS_FILE": str(results),
					"MANUAL_ANALYSIS_YAML": str(analysis),
					"MANUAL_ANALYSIS_LABEL": "//manual_analysis/example:manual_analysis",
					"MANUAL_ANALYSIS_LOBSTER_OUTPUT": str(lobster),
				},
				clear=False,
			):
				with self.assertRaises(SystemExit) as error:
					manual_analysis_test_runner.main(["--allow-check-failures"])
			self.assertEqual(getattr(error.exception, "code", None), 1)
			self.assertFalse(lobster.exists())


if __name__ == "__main__":
	unittest.main()

