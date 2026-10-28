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
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from lobster.common.errors import Message_Handler
from lobster.common.io import lobster_read

from manual_analysis.lobster_generator import generate_lobster_json


class LobsterGeneratorTest(unittest.TestCase):
    def test_canonical_labels_produce_readable_distinct_tags(self) -> None:
        cases = [
            (
                "@@//manual_analysis:received_input_pointer",
                "manualanalysis //manual_analysis:received_input_pointer",
            ),
            (
                "@@dependency+//manual_analysis:received_input_pointer",
                "manualanalysis dependency+//manual_analysis:received_input_pointer",
            ),
        ]
        items = {}
        with TemporaryDirectory() as directory:
            for index, (label, expected_tag) in enumerate(cases):
                with self.subTest(label=label):
                    json_content = generate_lobster_json(
                        requirements=["REQ-1"],
                        analysis_passed=True,
                        results_file_path="results.json",
                        analysis_label=label,
                    )
                    payload = json.loads(json_content)
                    tag = payload["data"][0]["tag"]
                    self.assertNotIn("@", tag)
                    self.assertEqual(tag, expected_tag)
                    self.assertIn(label, payload["data"][0]["name"])

                    output = Path(directory) / f"analysis_{index}.lobster"
                    output.write_text(json_content, encoding="utf-8")
                    lobster_read(Message_Handler(), str(output), "Manual Analysis", items)

        expected_keys = {expected_tag for label, expected_tag in cases}
        self.assertEqual(set(items), expected_keys)
        self.assertEqual(len(items), len(cases))
        for item in items.values():
            self.assertIsNone(item.tag.version)

    def test_generate_lobster_json_models_manual_verification(self) -> None:
        payload = json.loads(
            generate_lobster_json(
                requirements=["REQ-2", "REQ-1"],
                analysis_passed=True,
                results_file_path="manual_analysis/example/results.json",
                analysis_label="//manual_analysis/example:manual_analysis",
            )
        )

        self.assertEqual(payload["schema"], "lobster-act-trace")
        self.assertNotEqual(payload["schema"], "lobster-imp-trace")
        self.assertEqual(payload["generator"], "manual_analysis")
        self.assertEqual(payload["version"], 3)
        self.assertEqual(len(payload["data"]), 1)
        first = payload["data"][0]
        self.assertEqual(
            first["tag"],
            "manualanalysis //manual_analysis/example:manual_analysis",
        )
        self.assertEqual(first["status"], "ok")
        self.assertEqual(first["framework"], "manual_analysis")
        self.assertEqual(first["kind"], "Manual Analysis Run")
        self.assertEqual(first["location"]["file"], "manual_analysis/example/results.json")
        self.assertIsNone(first["location"]["line"])
        self.assertIsNone(first["location"]["column"])
        self.assertEqual(first["refs"], ["req REQ-1", "req REQ-2"])
        self.assertIn("Manual verification", first["name"])
        self.assertIn("manual_analysis", first["name"])

    def test_generate_lobster_json_failed_verification_message(self) -> None:
        payload = json.loads(
            generate_lobster_json(
                requirements=["REQ-1"],
                analysis_passed=False,
                results_file_path="results.json",
                analysis_label="//manual_analysis/example:manual_analysis",
            )
        )

        item = payload["data"][0]
        self.assertEqual(item["status"], "fail")


if __name__ == "__main__":
    unittest.main()
