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
from unittest import mock

from manual_analysis.interactive_runner_steps import (
    AnalysisFailedError,
    _execute_step,
)
from manual_analysis.yaml_schema import (
    ActionStep,
    AutomatedActionArg,
    AutomatedActionStep,
    RepeatStep,
    RepeatUntil,
)


class InteractiveRunnerStepsTest(unittest.TestCase):
    def test_execute_action_shows_instruction_and_result_in_history_entry(self) -> None:
        ui = mock.Mock()
        ui.prompt_multiline.return_value = "Observed output"
        results: list[dict] = []

        _execute_step(
            ActionStep(description="Run diagnostic command"),
            ui,
            results,
        )

        ui.print_header.assert_called_once_with("Manual Action")
        ui.show_text.assert_has_calls(
            [
                mock.call("Instructions", "Run diagnostic command"),
                mock.call("Result", "Observed output"),
            ]
        )
        ui.prompt_multiline.assert_called_once_with(
            "Run diagnostic command",
            initial_text="",
        )
        self.assertEqual(
            results,
            [
                {
                    "type": "action",
                    "description": "Run diagnostic command",
                    "result": "Observed output",
                }
            ],
        )

    def test_execute_automated_action_runs_command_and_records_result(self) -> None:
        ui = mock.Mock()
        ui.prompt_args_form.return_value = {"subject": "workspace"}
        ui.run_command.return_value = 0
        results: list[dict] = []

        _execute_step(
            AutomatedActionStep(
                command="echo {subject}",
                args=[AutomatedActionArg(name="subject", default="")],
                expected_return_code=0,
            ),
            ui,
            results,
        )

        ui.prompt_args_form.assert_called_once_with(
            [AutomatedActionArg(name="subject", default="")],
            initial_values=None,
        )
        ui.run_command.assert_called_once_with("echo workspace")
        self.assertEqual(results[0]["status"], "passed")
        self.assertEqual(results[0]["args"], {"subject": "workspace"})

    def test_execute_automated_action_raises_on_unexpected_return_code(self) -> None:
        ui = mock.Mock()
        ui.prompt_args_form.return_value = {"subject": "workspace"}
        ui.run_command.return_value = 2
        results: list[dict] = []

        with self.assertRaisesRegex(
            AnalysisFailedError,
            "expected return code 0, got 2",
        ):
            _execute_step(
                AutomatedActionStep(
                    command="echo {subject}",
                    args=[AutomatedActionArg(name="subject")],
                    expected_return_code=0,
                ),
                ui,
                results,
            )

        self.assertEqual(results[0]["status"], "failed")

    def test_execute_automated_action_preserves_explicit_empty_argument(self) -> None:
        ui = mock.Mock()
        ui.prompt_args_form.return_value = {"subject": ""}
        ui.run_command.return_value = 0
        results: list[dict] = []

        _execute_step(
            AutomatedActionStep(
                command="printf '%s' '{subject}'",
                args=[AutomatedActionArg(name="subject", default="value")],
                expected_return_code=0,
            ),
            ui,
            results,
        )

        ui.run_command.assert_called_once_with("printf '%s' ''")
        self.assertEqual(results[0]["args"]["subject"], "")

    def test_repeat_instruction_is_logged_only_when_repeat_prompt_is_shown(
        self,
    ) -> None:
        ui = mock.Mock()
        ui.prompt_multiline.return_value = "Iteration output"
        ui.prompt_choice.side_effect = ["Stop", "Yes"]
        results: list[dict] = []

        repeat_step = RepeatStep(
            until=RepeatUntil(
                description="Need another repetition?",
                continue_answer="Continue",
                break_answer="Stop",
            ),
            steps=[ActionStep(description="Run one iteration")],
        )

        _execute_step(repeat_step, ui, results)

        repeat_instruction_call = mock.call.show_text(
            "Instructions", "Need another repetition?"
        )
        self.assertEqual(ui.mock_calls.count(repeat_instruction_call), 1)

        repeat_instruction_index = ui.mock_calls.index(repeat_instruction_call)
        action_prompt_index = ui.mock_calls.index(
            mock.call.prompt_multiline("Run one iteration", initial_text="")
        )
        self.assertGreater(repeat_instruction_index, action_prompt_index)


if __name__ == "__main__":
    unittest.main()
