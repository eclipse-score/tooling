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
"""Schema parsing helpers for interactive manual analysis YAML files."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml


@dataclass(frozen=True)
class ActionStep:
    description: str


@dataclass(frozen=True)
class AutomatedActionArg:
    name: str
    default: str | None = None


@dataclass(frozen=True)
class AutomatedActionStep:
    command: str
    args: list[AutomatedActionArg]
    expected_return_code: int = 0


@dataclass(frozen=True)
class AssertionStep:
    description: str
    positive: str
    negative: str


@dataclass(frozen=True)
class DecisionBranch:
    answer: str
    steps: list[Step]


@dataclass(frozen=True)
class DecisionStep:
    description: str
    branches: list[DecisionBranch]


@dataclass(frozen=True)
class RepeatUntil:
    description: str
    continue_answer: str
    break_answer: str


@dataclass(frozen=True)
class RepeatStep:
    until: RepeatUntil
    steps: list[Step]


Step = ActionStep | AutomatedActionStep | AssertionStep | DecisionStep | RepeatStep


def _expect_dict(value: Any, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{context} must be a mapping")
    return value


def _expect_str(value: Any, context: str) -> str:
    if isinstance(value, bool):
        return "Yes" if value else "No"
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{context} must be a non-empty string")
    return value.strip()


def _parse_assertion(raw: dict[str, Any], context: str) -> AssertionStep:
    return AssertionStep(
        description=_expect_str(raw.get("description"), f"{context}.description"),
        positive=_expect_str(raw.get("positive"), f"{context}.positive"),
        negative=_expect_str(raw.get("negative"), f"{context}.negative"),
    )


def _expect_int(value: Any, context: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"{context} must be an integer")
    return value


def _expect_optional_str(value: Any, context: str) -> str | None:
    if value is None:
        return None
    if isinstance(value, bool):
        return "Yes" if value else "No"
    if not isinstance(value, str):
        raise ValueError(f"{context} must be a string when provided")
    return value


def _parse_automated_action_args(
    raw_args: Any, context: str
) -> list[AutomatedActionArg]:
    if raw_args is None:
        return []
    if not isinstance(raw_args, list):
        raise ValueError(f"{context}.args must be a list")

    args: list[AutomatedActionArg] = []
    for arg_idx, raw_arg in enumerate(raw_args):
        arg = _expect_dict(raw_arg, f"{context}.args[{arg_idx}]")
        args.append(
            AutomatedActionArg(
                name=_expect_str(arg.get("name"), f"{context}.args[{arg_idx}].name"),
                default=_expect_optional_str(
                    arg.get("default"),
                    f"{context}.args[{arg_idx}].default",
                ),
            )
        )
    return args


def _extract_step_payload(step: dict[str, Any], key: str) -> dict[str, Any]:
    payload = step.get(key)
    if isinstance(payload, dict):
        merged = dict(payload)
        for payload_key, value in step.items():
            if payload_key != key and payload_key not in merged:
                merged[payload_key] = value
        return merged
    return step


def _parse_step(raw_step: Any, index: int) -> Step:
    context = f"steps[{index}]"
    step = _expect_dict(raw_step, context)

    if "action" in step:
        return ActionStep(
            description=_expect_str(step.get("description"), f"{context}.description")
        )

    if "automated_action" in step:
        payload = _extract_step_payload(step, "automated_action")
        if "target" in payload:
            raise ValueError(f"{context}.target is no longer supported; use command")
        return AutomatedActionStep(
            command=_expect_str(payload.get("command"), f"{context}.command"),
            args=_parse_automated_action_args(payload.get("args"), context),
            expected_return_code=_expect_int(
                payload.get("expected_return_code", 0),
                f"{context}.expected_return_code",
            ),
        )

    if "decision" in step:
        description = _expect_str(step.get("description"), f"{context}.description")
        raw_branches = step.get("branches")
        if not isinstance(raw_branches, list) or not raw_branches:
            raise ValueError(f"{context}.branches must be a non-empty list")

        branches: list[DecisionBranch] = []
        for branch_idx, raw_branch in enumerate(raw_branches):
            branch = _expect_dict(raw_branch, f"{context}.branches[{branch_idx}]")
            branches.append(
                DecisionBranch(
                    answer=_expect_str(
                        branch.get("answer"),
                        f"{context}.branches[{branch_idx}].answer",
                    ),
                    steps=_parse_steps(
                        branch.get("steps"),
                        f"{context}.branches[{branch_idx}].steps",
                        allow_empty=True,
                    ),
                )
            )

        return DecisionStep(description=description, branches=branches)

    if "repeat" in step:
        payload = _extract_step_payload(step, "repeat")
        legacy_keys = []
        if "assertion-strategy" in payload:
            legacy_keys.append("assertion-strategy")
        if "assertion" in payload:
            legacy_keys.append("assertion")
        if legacy_keys:
            raise ValueError(
                f"{context} repeat no longer supports: {', '.join(legacy_keys)}"
            )
        until = _expect_dict(payload.get("until"), f"{context}.until")
        repeat_until = RepeatUntil(
            description=_expect_str(
                until.get("description"), f"{context}.until.description"
            ),
            continue_answer=_expect_str(
                until.get("continue"), f"{context}.until.continue"
            ),
            break_answer=_expect_str(until.get("break"), f"{context}.until.break"),
        )

        return RepeatStep(
            until=repeat_until,
            steps=_parse_steps(payload.get("steps"), f"{context}.steps"),
        )

    if "assertion" in step:
        return _parse_assertion(_extract_step_payload(step, "assertion"), context)

    raise ValueError(f"{context} has unknown step type. Keys: {sorted(step.keys())}")


def _parse_steps(
    raw_steps: Any,
    context: str = "steps",
    *,
    allow_empty: bool = False,
) -> list[Step]:
    if raw_steps is None and allow_empty:
        return []
    if not isinstance(raw_steps, list):
        raise ValueError(f"{context} must be a list")
    if not raw_steps and not allow_empty:
        raise ValueError(f"{context} must be a non-empty list")
    return [_parse_step(raw_step, index) for index, raw_step in enumerate(raw_steps)]


def _parse_requirements(raw_requirements: Any) -> list[str]:
    """Parse and validate the requirements list.
    
    Requirements must be a non-empty list of non-empty strings.
    """
    if raw_requirements is None:
        raise ValueError("requirements field is mandatory")
    if not isinstance(raw_requirements, list):
        raise ValueError("requirements must be a list")
    if not raw_requirements:
        raise ValueError("requirements must be a non-empty list")
    
    requirements = []
    for idx, req in enumerate(raw_requirements):
        if not isinstance(req, str) or not req.strip():
            raise ValueError(
                f"requirements[{idx}] must be a non-empty string, got: {req!r}"
            )
        requirements.append(req.strip())
    
    return requirements


def parse_analysis(data: Any) -> tuple[list[Step], list[str]]:
    """Parse analysis configuration and return steps and requirements.
    
    Returns:
        A tuple of (steps, requirements) where steps is a list of Step objects
        and requirements is a list of requirement identifier strings.
    """
    root = _expect_dict(data, "analysis")
    steps = _parse_steps(root.get("steps"), "steps")
    requirements = _parse_requirements(root.get("requirements"))

    if not isinstance(steps[-1], AssertionStep):
        raise ValueError("A manual analysis must end with an assertion step")

    return steps, requirements


def load_analysis(path: Path) -> tuple[list[Step], list[str]]:
    """Load and parse analysis YAML file.
    
    Returns:
        A tuple of (steps, requirements).
    """
    parsed = yaml.safe_load(path.read_text(encoding="utf-8"))
    return parse_analysis(parsed)
