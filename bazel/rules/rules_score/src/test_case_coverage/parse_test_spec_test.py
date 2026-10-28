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
"""Tests for parse_test_spec.py (RST test specification parser and CLI)."""

import json
import textwrap

import pytest

from test_case_coverage.compute_lock import TestStep
from test_case_coverage.parse_test_spec import TestSpecError, main, parse_test_spec

_REF = "//pkg:spec"

_VALID = textwrap.dedent(
    """\
    .. Copyright header comments are allowed.

    Connection fault handling
    =========================

    Test Goal
    ---------
    Show that a client survives an OS IPC failure.

    Second paragraph of the goal.

    Test Focus
    ----------
    - fault injection
    - error propagation

    Test Steps
    ----------

    Step 1: Connect
    ~~~~~~~~~~~~~~~
    :Precondition: server is running
    :Action: connect the client
    :Expected: the connection is established

    Step 2: Inject fault
    ~~~~~~~~~~~~~~~~~~~~
    :Action: let the next IPC call fail
             with EIO
    :Expected: the client receives an error
    """
)


def _without(section_title: str) -> str:
    """Return _VALID without the section *section_title* (up to the next same-level section)."""
    lines = _VALID.splitlines()
    start = lines.index(section_title)
    end = next(i for i in range(start + 2, len(lines)) if lines[i].startswith("Test ") or i == len(lines) - 1)
    return "\n".join(lines[:start] + lines[end:]) + "\n"


def test_valid_spec_is_parsed():
    spec = parse_test_spec(_VALID, _REF)
    assert spec.ref == _REF
    assert spec.goal == "Show that a client survives an OS IPC failure.\n\nSecond paragraph of the goal."
    assert spec.focus == "- fault injection\n- error propagation"
    assert spec.steps == (
        TestStep("server is running", "connect the client", "the connection is established"),
        TestStep("", "let the next IPC call fail with EIO", "the client receives an error"),
    )


def test_focus_is_optional():
    spec = parse_test_spec(_without("Test Focus"), _REF)
    assert spec.focus == ""


def test_missing_goal_fails():
    with pytest.raises(TestSpecError, match="'Test Goal' is missing"):
        parse_test_spec(_without("Test Goal"), _REF)


def test_missing_steps_fails():
    text = _VALID.split("Test Steps")[0]
    with pytest.raises(TestSpecError, match="'Test Steps' is missing"):
        parse_test_spec(text, _REF)


def test_unknown_section_fails():
    text = _VALID.replace("Test Focus\n----------", "Notes\n-----")
    with pytest.raises(TestSpecError, match="unknown section 'Notes'"):
        parse_test_spec(text, _REF)


def test_duplicate_section_fails():
    text = _VALID.replace("Test Focus\n----------", "Test Goal\n---------")
    with pytest.raises(TestSpecError, match="given more than once"):
        parse_test_spec(text, _REF)


def test_text_outside_sections_fails():
    text = _VALID.replace("Test Goal\n---------", "Some intro.\n\nTest Goal\n---------")
    with pytest.raises(TestSpecError, match="only the sections"):
        parse_test_spec(text, _REF)


def test_no_title_fails():
    with pytest.raises(TestSpecError, match="exactly one top-level section"):
        parse_test_spec("Just a paragraph.\n", _REF)


def test_step_without_expected_fails():
    text = _VALID.replace(":Expected: the client receives an error\n", "")
    with pytest.raises(TestSpecError, match=r"step 2 .*'Expected' is missing"):
        parse_test_spec(text, _REF)


def test_step_unknown_field_fails():
    text = _VALID.replace(":Precondition:", ":Setup:")
    with pytest.raises(TestSpecError, match="unknown field 'Setup'"):
        parse_test_spec(text, _REF)


def test_steps_without_subsections_fails():
    text = _VALID.split("Step 1")[0] + ":Action: a\n:Expected: e\n"
    with pytest.raises(TestSpecError, match="step subsections"):
        parse_test_spec(text, _REF)


def test_rst_syntax_problem_fails():
    text = _VALID.replace("Second paragraph of the goal.", ".. unknown-directive::")
    with pytest.raises(TestSpecError, match='Unknown directive type "unknown-directive"'):
        parse_test_spec(text, _REF)


def test_all_findings_are_reported():
    text = _VALID.replace("Test Focus\n----------", "Notes\n-----").replace(":Precondition:", ":Setup:")
    with pytest.raises(TestSpecError) as exc_info:
        parse_test_spec(text, _REF)
    assert len(exc_info.value.issues) == 2


def test_cli_writes_json(tmp_path):
    src = tmp_path / "spec.rst"
    src.write_text(_VALID, encoding="utf-8")
    out = tmp_path / "out" / "spec.test_spec.json"
    assert main(["--src", str(src), "--ref", _REF, "--alias", "//pkg:spec.rst", "--out", str(out)]) == 0
    data = json.loads(out.read_text(encoding="utf-8"))
    assert data["ref"] == _REF
    assert data["aliases"] == ["//pkg:spec.rst"]
    assert len(data["steps"]) == 2


def test_cli_fails_on_invalid_spec(tmp_path, capsys):
    src = tmp_path / "spec.rst"
    src.write_text(_without("Test Goal"), encoding="utf-8")
    out = tmp_path / "spec.test_spec.json"
    assert main(["--src", str(src), "--ref", _REF, "--out", str(out)]) == 1
    assert not out.exists()
    assert "invalid test specification //pkg:spec" in capsys.readouterr().err


if __name__ == "__main__":
    import sys

    sys.exit(pytest.main([__file__, "-v"]))
