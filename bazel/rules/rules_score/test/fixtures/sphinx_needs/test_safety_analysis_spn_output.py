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
"""Checks the LOBSTER items and the page safety_analysis_spn builds from fmea_needs.json."""

import os
import unittest

from lobster.common.errors import Message_Handler
from lobster.common.io import lobster_read


def _files():
    return {os.path.basename(path): path for path in os.environ["SAFETY_ANALYSIS_FILES"].split()}


def _items(path):
    items = {}
    lobster_read(Message_Handler(), path, "requirements", items)
    return {item.tag.tag: item for item in items.values()}


def _refs(item):
    return sorted(str(ref) for ref in item.unresolved_references)


class SafetyAnalysisSpnOutputTest(unittest.TestCase):
    def setUp(self):
        self.files = _files()

    def test_failure_modes_of_the_component_trace_to_api_methods(self):
        items = _items(self.files["failuremodes.lobster"])
        # comp_saf_fmea__bits__retired is invalid, comp_saf_fmea__other__crash violates another component.
        self.assertEqual(
            sorted(items), ["BitsFmea.comp_saf_fmea__bits__no_return", "BitsFmea.comp_saf_fmea__bits__wrong_bit"]
        )
        no_return = items["BitsFmea.comp_saf_fmea__bits__no_return"]
        self.assertEqual(no_return.kind, "FailureMode")
        refs = _refs(no_return)
        self.assertEqual(len(refs), 2)
        self.assertTrue(all(ref.startswith("req bits_api.") for ref in refs), refs)

    def test_root_causes_trace_to_their_failure_mode(self):
        items = _items(self.files["fta_root_causes.lobster"])
        self.assertEqual(
            {name.split(".", 1)[1]: _refs(item) for name, item in items.items()},
            {
                "CallNotIssued": ["req BitsFmea.comp_saf_fmea__bits__no_return"],
                "ExecutionLost": ["req BitsFmea.comp_saf_fmea__bits__no_return"],
                "PositionOutOfRange": ["req BitsFmea.comp_saf_fmea__bits__wrong_bit"],
            },
        )

    def test_only_sufficient_failure_modes_assess_their_root_causes(self):
        items = _items(self.files["assessments.lobster"])
        self.assertEqual(list(items), ["BitsFmea.comp_saf_fmea__bits__no_return.assessment"])
        refs = _refs(items["BitsFmea.comp_saf_fmea__bits__no_return.assessment"])
        # aou_req__platform__flow is not a measure of the analysis.
        self.assertEqual(
            [ref for ref in refs if not ref.startswith("req BitsFmea_fta.")],
            ["req aou_req__bits__integral@1"],
        )
        self.assertEqual(len(refs), 3)

    def test_page_shows_failure_modes_and_fault_trees(self):
        with open(self.files["safety_analysis.rst"], encoding="utf-8") as f:
            page = f.read()
        self.assertIn("``comp_saf_fmea__bits__no_return``", page)
        self.assertIn(".. uml:: fta_no_return.puml", page)
        self.assertIn(".. uml:: fta_wrong_bit.puml", page)


if __name__ == "__main__":
    unittest.main()
