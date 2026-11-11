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
"""Checks the needs.json exported by a dependable_element with docs = False."""

import json
import os
import unittest


class DependableElementNeedsJsonTest(unittest.TestCase):
    def setUp(self):
        with open(os.environ["NEEDS_JSON"], encoding="utf-8") as f:
            self.data = json.load(f)
        self.needs = self.data["versions"][self.data["current_version"]]["needs"]

    def test_project_is_the_element(self):
        self.assertEqual(self.data["project"], "needs_export_example_lib")

    def test_test_cases_become_testcase_needs(self):
        self.assertTrue(self.needs, "expected the test cases of :test_component")
        for need_id, need in self.needs.items():
            self.assertEqual(need["type"], "testcase")
            self.assertTrue(need_id.startswith("testcase__" + need["name"] + "_"))
            self.assertIn(need["result"], ("passed", "failed", "skipped"))

    def test_test_cases_link_their_requirements(self):
        linked = [n for n in self.needs.values() if n["partially_verifies"] or n["fully_verifies"]]
        self.assertTrue(linked, "expected test cases tracing to requirements")


if __name__ == "__main__":
    unittest.main()
