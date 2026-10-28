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
"""Parse and validate a formal test specification written in RST.

Used by the ``test_specification`` rule as a build action: a malformed file
fails the build, a valid one is written as normalized JSON (see
``test_spec.py``) for the coverage runners.

Expected structure (section underline characters are free to choose)::

    <Title of the test specification>
    =================================

    Test Goal
    ---------
    <text, mandatory>

    Test Focus
    ----------
    <text, optional section>

    Test Steps
    ----------

    Step 1: <short title>
    ~~~~~~~~~~~~~~~~~~~~~
    :Precondition: <optional>
    :Action: <mandatory>
    :Expected: <mandatory>

Comments (``.. ...``) are allowed anywhere; any other content outside of the
sections above is an error, so that everything a reviewer reads in the file
also ends up in the coverage lock.
"""

from __future__ import annotations

import argparse
import io
import json
import sys
from pathlib import Path

from docutils import nodes
from docutils.core import publish_doctree

from test_case_coverage.compute_lock import TestSpec, TestStep, spec_to_dict

_SECTION_GOAL = "test goal"
_SECTION_FOCUS = "test focus"
_SECTION_STEPS = "test steps"
_KNOWN_SECTIONS = (_SECTION_GOAL, _SECTION_FOCUS, _SECTION_STEPS)
_STEP_FIELDS = ("precondition", "action", "expected")


class TestSpecError(ValueError):
    """Raised with all findings when an RST test specification is invalid."""

    __test__ = False  # not a pytest test class

    def __init__(self, issues: list[str]) -> None:
        super().__init__("\n".join(issues))
        self.issues = issues


def _norm(text: str) -> str:
    return " ".join(text.split())


def _title(section: nodes.section) -> str:
    return _norm(section[0].astext()) if section.children and isinstance(section[0], nodes.title) else ""


def _content(section: nodes.Element) -> list[nodes.Node]:
    """Children of *section* without its title, comments and parser messages.

    Parser messages of level WARNING and above are reported separately via the
    warning stream; lower ones (INFO) are irrelevant for the structure.
    """
    return [c for c in section.children if not isinstance(c, (nodes.title, nodes.comment, nodes.system_message))]


def _block_text(node: nodes.Node) -> str:
    """Render a body element as plain text; list items keep their markers."""
    if isinstance(node, nodes.bullet_list):
        return "\n".join(f"- {_norm(item.astext())}" for item in node.children)
    if isinstance(node, nodes.enumerated_list):
        return "\n".join(f"{i}. {_norm(item.astext())}" for i, item in enumerate(node.children, start=1))
    return _norm(node.astext())


def _body_text(blocks: list[nodes.Node]) -> str:
    return "\n\n".join(t for t in (_block_text(b) for b in blocks) if t)


def _parse_step(step: nodes.section, index: int, issues: list[str]) -> TestStep:
    where = f"step {index} ({_title(step)!r})"
    content = _content(step)
    if len(content) != 1 or not isinstance(content[0], nodes.field_list):
        issues.append(f"{where}: must contain exactly one field list with :Precondition:, :Action:, :Expected:")
        return TestStep()

    values: dict[str, str] = {}
    for field in content[0].children:
        name = _norm(field[0].astext()).lower()
        if name not in _STEP_FIELDS:
            issues.append(f"{where}: unknown field {field[0].astext()!r} (allowed: Precondition, Action, Expected)")
            continue
        if name in values:
            issues.append(f"{where}: field {name.capitalize()!r} given more than once")
            continue
        values[name] = _body_text(list(field[1].children))

    for name in ("action", "expected"):
        if not values.get(name):
            issues.append(f"{where}: mandatory field {name.capitalize()!r} is missing or empty")
    return TestStep(
        precondition=values.get("precondition", ""),
        action=values.get("action", ""),
        expected=values.get("expected", ""),
    )


def _parse_steps(section: nodes.section, issues: list[str]) -> tuple[TestStep, ...]:
    steps: list[TestStep] = []
    for node in _content(section):
        if not isinstance(node, nodes.section):
            issues.append("'Test Steps' may only contain step subsections (e.g. 'Step 1: ...')")
            continue
        steps.append(_parse_step(node, len(steps) + 1, issues))
    if not steps:
        issues.append("'Test Steps' must contain at least one step subsection")
    return tuple(steps)


def parse_test_spec(text: str, ref: str) -> TestSpec:
    """Parse RST *text* of a test specification identified by label *ref*.

    Raises:
        TestSpecError: listing every finding (RST syntax and structure).
    """
    warnings = io.StringIO()
    doctree = publish_doctree(
        text,
        settings_overrides={
            "doctitle_xform": False,
            "report_level": 2,  # collect warnings and errors ...
            "halt_level": 5,  # ... without aborting on the first one
            "warning_stream": warnings,
            "file_insertion_enabled": False,
            "raw_enabled": False,
        },
    )
    issues = [line for line in warnings.getvalue().splitlines() if line.strip()]

    top = _content(doctree)
    if len(top) != 1 or not isinstance(top[0], nodes.section):
        issues.append("the file must contain exactly one top-level section (the title of the test specification)")
        raise TestSpecError(issues)

    sections: dict[str, nodes.section] = {}
    for node in _content(top[0]):
        if not isinstance(node, nodes.section):
            issues.append(
                "only the sections 'Test Goal', 'Test Focus' and 'Test Steps' are allowed below the title, "
                f"found text {_norm(node.astext())[:60]!r}"
            )
            continue
        name = _title(node).lower()
        if name not in _KNOWN_SECTIONS:
            issues.append(f"unknown section {_title(node)!r} (allowed: Test Goal, Test Focus, Test Steps)")
        elif name in sections:
            issues.append(f"section {_title(node)!r} given more than once")
        else:
            sections[name] = node

    goal = _body_text(_content(sections[_SECTION_GOAL])) if _SECTION_GOAL in sections else ""
    if not goal:
        issues.append("mandatory section 'Test Goal' is missing or empty")
    focus = _body_text(_content(sections[_SECTION_FOCUS])) if _SECTION_FOCUS in sections else ""
    if _SECTION_STEPS in sections:
        steps = _parse_steps(sections[_SECTION_STEPS], issues)
    else:
        issues.append("mandatory section 'Test Steps' is missing")
        steps = ()

    if issues:
        raise TestSpecError(issues)
    return TestSpec(ref=ref, goal=goal, focus=focus, steps=steps)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--src", required=True, type=Path, help="RST file of the test specification")
    parser.add_argument("--ref", required=True, help="Bazel label of the test_specification target")
    parser.add_argument("--alias", action="append", default=[], help="Additional label referring to the spec")
    parser.add_argument("--out", required=True, type=Path, help="Normalized JSON output")
    args = parser.parse_args(argv)

    try:
        spec = parse_test_spec(args.src.read_text(encoding="utf-8"), args.ref)
    except TestSpecError as exc:
        print(f"ERROR: invalid test specification {args.ref} ({args.src}):", file=sys.stderr)
        for issue in exc.issues:
            print(f"  - {issue}", file=sys.stderr)
        return 1

    data = spec_to_dict(spec)
    data["aliases"] = args.alias
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
