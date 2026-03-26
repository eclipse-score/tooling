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

from manual_analysis.interactive_runner_ui_console import _ConsoleUI
from manual_analysis.yaml_schema import AutomatedActionArg


class InteractiveRunnerUiConsoleTest(unittest.TestCase):
    def test_show_text_uses_pager_for_large_content(self) -> None:
        ui = _ConsoleUI(lambda _: "")
        large_content = "x" * 1700

        with (
            mock.patch(
                "manual_analysis.interactive_runner_ui_console.pydoc.pager"
            ) as pager,
            mock.patch("builtins.print") as print_mock,
        ):
            ui.show_text("Large", large_content)

        pager.assert_called_once()
        print_mock.assert_not_called()

    def test_prompt_choice_retries_until_valid_answer(self) -> None:
        answers = iter(["invalid", "yes"])
        ui = _ConsoleUI(lambda _: next(answers))

        with mock.patch("builtins.print") as print_mock:
            selected = ui.prompt_choice("Continue?", ["Yes", "No"])

        self.assertEqual(selected, "Yes")
        self.assertTrue(
            any("Invalid answer" in str(call) for call in print_mock.mock_calls)
        )

    def test_prompt_multiline_keeps_initial_text(self) -> None:
        answers = iter(["line 2", "\x01"])
        ui = _ConsoleUI(lambda _: next(answers))

        value = ui.prompt_multiline("Describe", initial_text="line 1")

        self.assertEqual(value, "line 1\nline 2")

    def test_prompt_args_form_prefills_values(self) -> None:
        answers = iter(["new-subject", ""])
        ui = _ConsoleUI(lambda _: next(answers))

        values = ui.prompt_args_form(
            [
                AutomatedActionArg(name="subject", default="old"),
                AutomatedActionArg(name="mode", default="fast"),
            ],
            initial_values={"subject": "prefill"},
        )

        self.assertEqual(values, {"subject": "new-subject", "mode": ""})


if __name__ == "__main__":
    unittest.main()
