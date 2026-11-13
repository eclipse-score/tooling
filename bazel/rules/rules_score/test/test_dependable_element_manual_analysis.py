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

"""Integration tests for manual analysis evidence in dependable elements."""

import contextlib
import io
import json
import os
import unittest
from unittest.mock import patch

from manual_analysis.common import resolve_path
from manual_analysis.manual_analysis_test_runner import main as run_review


class TestDependableElementManualAnalysis(unittest.TestCase):
    def setUp(self) -> None:
        self.configuration = json.loads(resolve_path(os.environ["MANUAL_ANALYSIS_CONFIG"]).read_text())
        self.expected_error = self.configuration["MANUAL_ANALYSIS_EXPECTED_ERROR"]
        self.expected_status = "fail" if self.expected_error else "ok"

    def test_review_result(self) -> None:
        output = io.StringIO()
        with patch.dict(os.environ, self.configuration), contextlib.redirect_stderr(output):
            if self.expected_error:
                with self.assertRaises(SystemExit) as error:
                    run_review([])
                self.assertEqual(error.exception.code, 1)
            else:
                run_review([])
        if self.expected_error:
            self.assertIn(self.expected_error, output.getvalue())
        else:
            self.assertEqual(output.getvalue(), "")

    def test_generated_evidence(self) -> None:
        evidence = json.loads(resolve_path(self.configuration["MANUAL_ANALYSIS_EVIDENCE"]).read_text())
        self.assertEqual(len(evidence["data"]), 1)
        self.assertEqual(evidence["data"][0]["status"], self.expected_status)

    def test_aggregated_report(self) -> None:
        evidence = json.loads(resolve_path(self.configuration["MANUAL_ANALYSIS_EVIDENCE"]).read_text())
        report = json.loads(resolve_path(self.configuration["MANUAL_ANALYSIS_REPORT"]).read_text())
        levels = {level["name"]: level for level in report["levels"]}
        activities = levels["Manual Analysis"]["items"]
        self.assertEqual(len(activities), 1)
        activity = activities[0]
        self.assertEqual(len(evidence["data"]), 1)
        self.assertEqual(activity["tag"], evidence["data"][0]["tag"])
        self.assertEqual(activity["status"], self.expected_status)
        self.assertEqual(activity["tracing_status"], "OK")
        received = levels["Received AoUs"]["items"]
        self.assertEqual(len(received), 1)
        self.assertEqual(received[0]["ref_down"], [activity["tag"]])
        self.assertEqual(received[0]["tracing_status"], "OK")


if __name__ == "__main__":
    unittest.main()
