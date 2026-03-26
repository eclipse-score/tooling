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
from manual_analysis.yaml_schema import (
    ActionStep,
    AssertionStep,
    AutomatedActionArg,
    AutomatedActionStep,
    DecisionStep,
    RepeatStep,
    parse_analysis,
)


class YamlSchemaTest(unittest.TestCase):
    def test_parse_supported_steps(self) -> None:
        steps, requirements = parse_analysis(
            {
                "requirements": ["REQ-001", "REQ-002"],
                "steps": [
                    {"action": None, "description": "Collect findings"},
                    {
                        "automated_action": None,
                        "command": "echo {name}",
                        "args": [
                            {"name": "name", "default": "world"},
                            {"name": "suffix"},
                        ],
                        "expected_return_code": 7,
                    },
                    {
                        "repeat": None,
                        "until": {
                            "description": "Repeat?",
                            "continue": "Yes",
                            "break": "No",
                        },
                        "steps": [
                            {
                                "decision": None,
                                "description": "Continue triage?",
                                "branches": [
                                    {
                                        "answer": "Yes",
                                        "steps": [
                                            {
                                                "action": None,
                                                "description": "Inspect logs",
                                            }
                                        ],
                                    },
                                    {
                                        "answer": "No",
                                        "steps": [
                                            {
                                                "action": None,
                                                "description": "Skip logs",
                                            }
                                        ],
                                    },
                                ],
                            }
                        ],
                    },
                    {
                        "assertion": None,
                        "description": "Any unresolved issues?",
                        "positive": "No",
                        "negative": "Yes",
                    },
                ]
            }
        )

        self.assertIsInstance(steps[0], ActionStep)
        self.assertIsInstance(steps[1], AutomatedActionStep)
        self.assertEqual(steps[1].command, "echo {name}")
        self.assertEqual(
            steps[1].args,
            [
                AutomatedActionArg(name="name", default="world"),
                AutomatedActionArg(name="suffix", default=None),
            ],
        )
        self.assertEqual(steps[1].expected_return_code, 7)
        self.assertIsInstance(steps[2], RepeatStep)
        self.assertIsInstance(steps[2].steps[0], DecisionStep)
        self.assertIsInstance(steps[3], AssertionStep)
        self.assertEqual(requirements, ["REQ-001", "REQ-002"])

    def test_repeat_rejects_removed_assertion_keys(self) -> None:
        with self.assertRaisesRegex(
            ValueError,
            "repeat no longer supports: assertion-strategy, assertion",
        ):
            parse_analysis(
                {
                    "requirements": ["REQ-001"],
                    "steps": [
                        {
                            "repeat": None,
                            "until": {
                                "description": "Repeat?",
                                "continue": "Yes",
                                "break": "No",
                            },
                            "assertion-strategy": "once-at-end",
                            "steps": [{"action": None, "description": "Inspect logs"}],
                            "assertion": {
                                "description": "Any errors?",
                                "positive": "No",
                                "negative": "Yes",
                            },
                        },
                        {
                            "assertion": None,
                            "description": "Any unresolved issues?",
                            "positive": "No",
                            "negative": "Yes",
                        },
                    ]
                }
            )

    def test_automated_action_defaults_expected_return_code(self) -> None:
        steps, _ = parse_analysis(
            {
                "requirements": ["REQ-001"],
                "steps": [
                    {"automated_action": None, "command": "true", "args": []},
                    {
                        "assertion": None,
                        "description": "Any unresolved issues?",
                        "positive": "No",
                        "negative": "Yes",
                    },
                ]
            }
        )
        self.assertEqual(steps[0].expected_return_code, 0)

    def test_automated_action_rejects_legacy_target(self) -> None:
        with self.assertRaisesRegex(ValueError, "target is no longer supported"):
            parse_analysis(
                {
                    "requirements": ["REQ-001"],
                    "steps": [
                        {"automated_action": None, "target": "//demo:auto"},
                        {
                            "assertion": None,
                            "description": "Any unresolved issues?",
                            "positive": "No",
                            "negative": "Yes",
                        },
                    ]
                }
            )

    def test_decision_branch_allows_empty_or_missing_steps(self) -> None:
        steps, _ = parse_analysis(
            {
                "requirements": ["REQ-001"],
                "steps": [
                    {
                        "decision": None,
                        "description": "Continue triage?",
                        "branches": [
                            {"answer": "Yes", "steps": []},
                            {"answer": "No"},
                        ],
                    },
                    {
                        "assertion": None,
                        "description": "Any unresolved issues?",
                        "positive": "No",
                        "negative": "Yes",
                    },
                ]
            }
        )

        self.assertEqual(steps[0].branches[0].steps, [])
        self.assertEqual(steps[0].branches[1].steps, [])

    def test_decision_branch_rejects_invalid_steps_type(self) -> None:
        with self.assertRaisesRegex(
            ValueError,
            r"steps\[0\]\.branches\[0\]\.steps must be a list",
        ):
            parse_analysis(
                {
                    "requirements": ["REQ-001"],
                    "steps": [
                        {
                            "decision": None,
                            "description": "Continue triage?",
                            "branches": [
                                {"answer": "Yes", "steps": "invalid"},
                            ],
                        },
                        {
                            "assertion": None,
                            "description": "Any unresolved issues?",
                            "positive": "No",
                            "negative": "Yes",
                        },
                    ]
                }
            )

    def test_requires_final_assertion(self) -> None:
        with self.assertRaisesRegex(ValueError, "must end with an assertion"):
            parse_analysis({
                "requirements": ["REQ-001"],
                "steps": [{"action": None, "description": "Only action"}]
            })

    def test_requires_requirements_field(self) -> None:
        with self.assertRaisesRegex(ValueError, "requirements field is mandatory"):
            parse_analysis({"steps": [
                {"action": None, "description": "Action"},
                {"assertion": None, "description": "Test", "positive": "Yes", "negative": "No"}
            ]})

    def test_requires_non_empty_requirements(self) -> None:
        with self.assertRaisesRegex(ValueError, "requirements must be a non-empty list"):
            parse_analysis({
                "requirements": [],
                "steps": [
                    {"action": None, "description": "Action"},
                    {"assertion": None, "description": "Test", "positive": "Yes", "negative": "No"}
                ]
            })


if __name__ == "__main__":
    unittest.main()
