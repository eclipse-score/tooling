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
"""Previous-run prefill state for interactive manual analyses."""

from __future__ import annotations

import json
from pathlib import Path


class _PrefillState:
    def __init__(self) -> None:
        self._action_by_description: dict[str, list[str]] = {}
        self._assertion_by_description: dict[str, list[str]] = {}
        self._assertion_justification_by_description: dict[str, list[str]] = {}
        self._decision_by_description: dict[str, list[str]] = {}
        self._decision_justification_by_description: dict[str, list[str]] = {}
        self._automated_args_by_template: dict[str, list[dict[str, str]]] = {}
        self._repeat_until_by_description: dict[str, list[list[str]]] = {}
        self._repeat_iteration_count_by_description: dict[str, list[int]] = {}

    @staticmethod
    def load(results_path: Path) -> "_PrefillState":
        state = _PrefillState()
        if not results_path.exists():
            return state
        try:
            payload = json.loads(results_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return state
        if not isinstance(payload, dict):
            return state
        raw_results = payload.get("results")
        if not isinstance(raw_results, list):
            return state
        for entry in raw_results:
            state._collect_entry(entry)
        return state

    def _push(self, store: dict[str, list], key: str, value) -> None:  # type: ignore[no-untyped-def]
        if key not in store:
            store[key] = []
        store[key].append(value)

    def _pop(self, store: dict[str, list], key: str):  # type: ignore[no-untyped-def]
        values = store.get(key)
        if not values:
            return None
        return values.pop(0)

    def _collect_entry(self, entry) -> None:  # type: ignore[no-untyped-def]
        if not isinstance(entry, dict):
            return
        entry_type = entry.get("type")

        if entry_type == "action":
            description = entry.get("description")
            result = entry.get("result")
            if isinstance(description, str) and isinstance(result, str):
                self._push(self._action_by_description, description, result)
            return

        if entry_type == "automated_action":
            template = entry.get("command_template")
            args = entry.get("args")
            if isinstance(template, str) and isinstance(args, dict):
                normalized = {str(name): str(value) for name, value in args.items()}
                self._push(self._automated_args_by_template, template, normalized)
            return

        if entry_type == "assertion":
            description = entry.get("description")
            answer = entry.get("answer")
            if isinstance(description, str) and isinstance(answer, str):
                self._push(self._assertion_by_description, description, answer)
            justification = entry.get("justification")
            if isinstance(description, str) and isinstance(justification, str):
                self._push(
                    self._assertion_justification_by_description,
                    description,
                    justification,
                )
            return

        if entry_type == "decision":
            description = entry.get("description")
            answer = entry.get("answer")
            if isinstance(description, str) and isinstance(answer, str):
                self._push(self._decision_by_description, description, answer)
            justification = entry.get("justification")
            if isinstance(description, str) and isinstance(justification, str):
                self._push(
                    self._decision_justification_by_description,
                    description,
                    justification,
                )
            nested_steps = entry.get("steps")
            if isinstance(nested_steps, list):
                for nested in nested_steps:
                    self._collect_entry(nested)
            return

        if entry_type == "repeat":
            until_description = entry.get("until")
            until_answers = entry.get("until_answers")
            if isinstance(until_description, str) and isinstance(until_answers, list):
                normalized_answers = [a for a in until_answers if isinstance(a, str)]
                self._push(
                    self._repeat_until_by_description,
                    until_description,
                    normalized_answers,
                )

            iterations = entry.get("iterations")
            if isinstance(iterations, list):
                if isinstance(until_description, str):
                    self._push(
                        self._repeat_iteration_count_by_description,
                        until_description,
                        len(iterations),
                    )
                for iteration in iterations:
                    if isinstance(iteration, list):
                        for nested in iteration:
                            self._collect_entry(nested)
            final_assertion = entry.get("final_assertion")
            if isinstance(final_assertion, dict):
                self._collect_entry(final_assertion)
            return

    def next_action(self, description: str) -> str | None:
        value = self._pop(self._action_by_description, description)
        return value if isinstance(value, str) else None

    def next_assertion(self, description: str, options: list[str]) -> str | None:
        value = self._pop(self._assertion_by_description, description)
        return value if isinstance(value, str) and value in options else None

    def next_assertion_justification(self, description: str) -> str | None:
        value = self._pop(self._assertion_justification_by_description, description)
        return value if isinstance(value, str) else None

    def next_decision(self, description: str, options: list[str]) -> str | None:
        value = self._pop(self._decision_by_description, description)
        return value if isinstance(value, str) and value in options else None

    def next_decision_justification(self, description: str) -> str | None:
        value = self._pop(self._decision_justification_by_description, description)
        return value if isinstance(value, str) else None

    def next_automated_args(
        self,
        command_template: str,
        arg_names: list[str],
    ) -> dict[str, str] | None:
        value = self._pop(self._automated_args_by_template, command_template)
        if not isinstance(value, dict):
            return None
        return {name: str(value[name]) for name in arg_names if name in value}

    def next_repeat_until_answers(
        self,
        description: str,
        continue_answer: str,
        break_answer: str,
    ) -> list[str] | None:
        value = self._pop(self._repeat_until_by_description, description)
        if isinstance(value, list):
            return [v for v in value if isinstance(v, str)]
        iteration_count = self._pop(
            self._repeat_iteration_count_by_description, description
        )
        if isinstance(iteration_count, int) and iteration_count > 0:
            if iteration_count == 1:
                return [break_answer]
            return [continue_answer] * (iteration_count - 1) + [break_answer]
        return None
