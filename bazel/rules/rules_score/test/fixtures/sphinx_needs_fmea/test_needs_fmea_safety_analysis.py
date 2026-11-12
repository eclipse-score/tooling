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
"""Checks the assessments a safety_analysis derives from Sphinx-Needs FMEA needs."""

import json
import os
import unittest


class NeedsFmeaSafetyAnalysisTest(unittest.TestCase):
    def test_assessment_traces_root_cause_and_measure(self):
        paths = [p for p in os.environ["SAFETY_ANALYSIS_FILES"].split() if p.endswith("/assessments.lobster")]
        self.assertEqual(1, len(paths), os.environ["SAFETY_ANALYSIS_FILES"])
        with open(paths[0], encoding="utf-8") as f:
            items = json.load(f)["data"]
        self.assertEqual(
            ["req NeedsFmeaExample.comp_saf_fmea__example__no_response.assessment"],
            [i["tag"].split("@")[0] for i in items],
        )
        self.assertEqual(
            ["req NeedsFmeaFta.ProviderProcessCrashed", "req comp_req__example__retry@1"],
            sorted(items[0]["refs"]),
        )


if __name__ == "__main__":
    unittest.main()
