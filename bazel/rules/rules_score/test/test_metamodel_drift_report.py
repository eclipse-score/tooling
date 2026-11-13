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
"""Report-only smoke test against the real requirements model and metamodel."""

from __future__ import annotations

import contextlib
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

from metamodel_drift import main


class MetamodelDriftReportTest(unittest.TestCase):
    def test_real_models_produce_a_valid_report(self) -> None:
        arguments = sys.argv[1:]
        runfiles = Path(os.environ["TEST_SRCDIR"])
        path_flags = {"--rsl", "--metamodel", "--mapping"}
        for index, argument in enumerate(arguments[:-1]):
            if argument in path_flags and not Path(arguments[index + 1]).is_absolute():
                arguments[index + 1] = str(runfiles / arguments[index + 1])

        with tempfile.TemporaryDirectory() as directory:
            report_path = Path(directory) / "report.json"
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                exit_code = main(arguments + ["--report-json", str(report_path)])
            self.assertEqual(0, exit_code)
            self.assertIn("# Metamodel Drift Report", stdout.getvalue())
            report = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertIsInstance(report["findings"], list)
            self.assertEqual({"error", "warning", "info"}, set(report["summary"]))


if __name__ == "__main__":
    unittest.main(argv=[sys.argv[0]])
