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
"""Step execution logic for the interactive manual-analysis runner."""

from __future__ import annotations

from typing import Protocol

from manual_analysis.interactive_runner_prefill import _PrefillState
from manual_analysis.yaml_schema import (
    ActionStep,
    AssertionStep,
    AutomatedActionStep,
    DecisionStep,
    RepeatStep,
    Step,
)


class AnalysisFailedError(RuntimeError):
    """Raised when an assertion result marks the analysis as failed."""


class _RunnerUI(Protocol):
    def print_header(self, title: str) -> None: ...

    def show_text(self, title: str, content: str) -> None: ...

    def prompt_choice(
        self,
        description: str,
        options: list[str],
        default_option: str | None = None,
    ) -> str: ...

    def prompt_choice_with_justification(
        self,
        description: str,
        options: list[str],
        default_option: str | None = None,
        default_justification: str | None = None,
    ) -> tuple[str, str]: ...

    def prompt_multiline(self, prompt: str, initial_text: str = "") -> str: ...

    def prompt_args_form(
        self,
        args,
        initial_values: dict[str, str] | None = None,
    ) -> dict[str, str]: ...

    def run_command(self, command: str) -> int: ...


def _execute_step(
    step: Step,
    ui: _RunnerUI,
    results: list[dict],
    prefill: _PrefillState | None = None,
) -> None:
    if isinstance(step, ActionStep):
        ui.print_header("Manual Action")
        ui.show_text("Instructions", step.description)
        initial_text = ""
        if prefill is not None:
            initial_text = prefill.next_action(step.description) or ""
        action_result = ui.prompt_multiline(step.description, initial_text=initial_text)
        ui.show_text("Result", action_result)
        results.append(
            {
                "type": "action",
                "description": step.description,
                "result": action_result,
            }
        )
        return

    if isinstance(step, AutomatedActionStep):
        ui.print_header("Automated Action")
        ui.show_text("Instructions", f"Command template:\n{step.command}")
        prefill_args = None
        if prefill is not None:
            prefill_args = prefill.next_automated_args(
                step.command,
                [arg.name for arg in step.args],
            )
        arg_values = ui.prompt_args_form(step.args, initial_values=prefill_args)
        try:
            command = step.command.format_map(arg_values)
        except KeyError as err:
            raise AnalysisFailedError(
                f"Automated action references undefined argument: {err}"
            ) from err

        ui.show_text("Resolved Command", command)
        return_code = ui.run_command(command)
        passed = return_code == step.expected_return_code
        ui.show_text(
            "Result",
            (
                f"Return code: {return_code} (expected {step.expected_return_code})"
                if not passed
                else f"Return code: {return_code}"
            ),
        )
        results.append(
            {
                "type": "automated_action",
                "command_template": step.command,
                "command": command,
                "args": arg_values,
                "expected_return_code": step.expected_return_code,
                "return_code": return_code,
                "status": "passed" if passed else "failed",
            }
        )
        if not passed:
            raise AnalysisFailedError(
                "Automated action failed: "
                f"expected return code {step.expected_return_code}, got {return_code}"
            )
        return

    if isinstance(step, AssertionStep):
        ui.print_header("Assertion")
        ui.show_text(
            "Instructions",
            f"{step.description}\n\nAllowed answers: {step.positive}, {step.negative}",
        )
        answer, justification = ui.prompt_choice_with_justification(
            step.description,
            [step.positive, step.negative],
            default_option=(
                prefill.next_assertion(step.description, [step.positive, step.negative])
                if prefill is not None
                else None
            ),
            default_justification=(
                prefill.next_assertion_justification(step.description)
                if prefill is not None
                else None
            ),
        )
        passed = answer == step.positive
        ui.show_text("Result", f"Answer: {answer}")
        assertion_result = {
            "type": "assertion",
            "description": step.description,
            "answer": answer,
            "passed": passed,
        }
        if justification:
            assertion_result["justification"] = justification
        results.append(assertion_result)
        if not passed:
            raise AnalysisFailedError(f"Assertion failed: {step.description}")
        return

    if isinstance(step, DecisionStep):
        ui.print_header("Decision")
        ui.show_text("Instructions", step.description)
        options = [branch.answer for branch in step.branches]
        selected_answer, justification = ui.prompt_choice_with_justification(
            step.description,
            options,
            default_option=(
                prefill.next_decision(step.description, options)
                if prefill is not None
                else None
            ),
            default_justification=(
                prefill.next_decision_justification(step.description)
                if prefill is not None
                else None
            ),
        )
        ui.show_text("Result", f"Selected branch: {selected_answer}")

        branch_result: list[dict] = []
        selected_branch = next(
            branch for branch in step.branches if branch.answer == selected_answer
        )
        for nested_step in selected_branch.steps:
            _execute_step(nested_step, ui, branch_result, prefill=prefill)

        decision_result = {
            "type": "decision",
            "description": step.description,
            "answer": selected_answer,
            "steps": branch_result,
        }
        if justification:
            decision_result["justification"] = justification
        results.append(decision_result)
        return

    if isinstance(step, RepeatStep):
        ui.print_header("Repeat")
        iterations: list[list[dict]] = []
        until_answers: list[str] = []
        prefill_until_answers: list[str] = []
        if prefill is not None:
            prefill_until_answers = (
                prefill.next_repeat_until_answers(
                    step.until.description,
                    step.until.continue_answer,
                    step.until.break_answer,
                )
                or []
            )

        while True:
            iteration_result: list[dict] = []
            for nested_step in step.steps:
                _execute_step(nested_step, ui, iteration_result, prefill=prefill)

            iterations.append(iteration_result)
            ui.show_text("Instructions", step.until.description)
            answer = ui.prompt_choice(
                step.until.description,
                [step.until.continue_answer, step.until.break_answer],
                default_option=(
                    prefill_until_answers[len(until_answers)]
                    if len(until_answers) < len(prefill_until_answers)
                    and prefill_until_answers[len(until_answers)]
                    in [step.until.continue_answer, step.until.break_answer]
                    else None
                ),
            )
            until_answers.append(answer)
            if answer == step.until.break_answer:
                break

        results.append(
            {
                "type": "repeat",
                "until": step.until.description,
                "until_answers": until_answers,
                "iterations": iterations,
            }
        )
        return

    raise ValueError(f"Unsupported step type: {type(step).__name__}")
