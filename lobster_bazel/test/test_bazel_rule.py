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
import unittest
from pathlib import Path

from test_runner import TestRunner


class BazelRuleTest(unittest.TestCase):
    """Tests for basic tracing tag extraction from source files."""

    def setUp(self):
        cwd = Path(__file__).parent
        # Resolve lobster_bazel binary from runfiles
        self._output_lobster: Path = cwd / "impl_trace.lobster"
        self._test_runner = TestRunner(cwd, cwd)

    def test_correct_source_generation(self):
        """
        Read source file and configrm that it contains expected file names.
        """

        expected_file_names = {
            "source_with_tags.py",
            "dependency_with_tags.py",
            "transitive_dependency_with_tags.py",
            "source_with_tags.h",
            "source_with_tags.cpp",
        }

        file_names = self._test_runner.extract_lobster_file_names(self._output_lobster)

        required_file_names = expected_file_names - file_names

        self.assertTrue(
            expected_file_names <= file_names,
            f"""Not all dependencies were extracted from the
targets. Required but not extracted file names: {required_file_names}""",
        )


if __name__ == "__main__":
    unittest.main()
