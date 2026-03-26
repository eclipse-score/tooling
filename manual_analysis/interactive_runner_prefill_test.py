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

from manual_analysis.interactive_runner_prefill import _PrefillState


class InteractiveRunnerPrefillTest(unittest.TestCase):
    def test_load_missing_file_returns_empty_state(self) -> None:
        state = _PrefillState.load(Path("/tmp/does-not-exist-results-file.json"))
        self.assertIsNone(state.next_action("missing"))

    def test_load_collects_and_returns_values_in_fifo_order(self) -> None:
        payload = {
            "results": [
                {"type": "action", "description": "collect", "result": "first"},
                {"type": "action", "description": "collect", "result": "second"},
                {
                    "type": "automated_action",
                    "command_template": "echo {name}",
                    "args": {"name": "workspace", "count": 7},
                },
                {
                    "type": "assertion",
                    "description": "ok?",
                    "answer": "No",
                    "justification": "validated by check",
                },
                {
                    "type": "decision",
                    "description": "branch?",
                    "answer": "A",
                    "justification": "matches scenario",
                    "steps": [
                        {"type": "action", "description": "nested", "result": "inside"}
                    ],
                },
                {
                    "type": "repeat",
                    "until": "again?",
                    "until_answers": ["Continue", "Stop", 42],
                    "iterations": [
                        [{"type": "action", "description": "iter", "result": "i1"}]
                    ],
                    "final_assertion": {
                        "type": "assertion",
                        "description": "final",
                        "answer": "No",
                        "justification": "final check is clean",
                    },
                },
            ]
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            results_path = Path(tmpdir) / "results.json"
            results_path.write_text(json.dumps(payload), encoding="utf-8")
            state = _PrefillState.load(results_path)

        self.assertEqual(state.next_action("collect"), "first")
        self.assertEqual(state.next_action("collect"), "second")
        self.assertEqual(
            state.next_automated_args("echo {name}", ["name", "count"]),
            {"name": "workspace", "count": "7"},
        )
        self.assertEqual(state.next_assertion("ok?", ["Yes", "No"]), "No")
        self.assertEqual(
            state.next_assertion_justification("ok?"), "validated by check"
        )
        self.assertEqual(state.next_decision("branch?", ["A", "B"]), "A")
        self.assertEqual(
            state.next_decision_justification("branch?"), "matches scenario"
        )
        self.assertEqual(state.next_action("nested"), "inside")
        self.assertEqual(
            state.next_repeat_until_answers("again?", "Continue", "Stop"),
            ["Continue", "Stop"],
        )
        self.assertEqual(state.next_action("iter"), "i1")
        self.assertEqual(state.next_assertion("final", ["No", "Yes"]), "No")
        self.assertEqual(
            state.next_assertion_justification("final"), "final check is clean"
        )

    def test_repeat_until_falls_back_to_iteration_count(self) -> None:
        payload = {
            "results": [
                {
                    "type": "repeat",
                    "until": "again?",
                    "iterations": [[], [], []],
                }
            ]
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            results_path = Path(tmpdir) / "results.json"
            results_path.write_text(json.dumps(payload), encoding="utf-8")
            state = _PrefillState.load(results_path)

        self.assertEqual(
            state.next_repeat_until_answers("again?", "Continue", "Stop"),
            ["Continue", "Continue", "Stop"],
        )


if __name__ == "__main__":
    unittest.main()
