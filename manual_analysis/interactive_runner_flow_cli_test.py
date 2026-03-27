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
import os
import tempfile
import unittest
from pathlib import Path
from typing import TYPE_CHECKING, cast
from unittest import mock

from manual_analysis import interactive_runner_cli
from manual_analysis.interactive_runner_flow import run_analysis
from manual_analysis.interactive_runner_prefill import _PrefillState
from manual_analysis.interactive_runner_runtime import _workspace_root
from manual_analysis.yaml_schema import AutomatedActionArg
from manual_analysis.yaml_schema import load_analysis

if TYPE_CHECKING:
    from manual_analysis.interactive_runner_steps import _RunnerUI


class _FakeUi:
    def __init__(self, answers: list[str]) -> None:
        self._answers = iter(answers)

    def _next(self) -> str:
        try:
            return next(self._answers)
        except StopIteration as err:
            raise KeyboardInterrupt from err

    def print_header(self, _title: str) -> None:
        return

    def show_text(self, _title: str, _content: str) -> None:
        return

    def prompt_multiline(self, _prompt: str, initial_text: str = "") -> str:
        value = self._next()
        if value == "\x01":
            return initial_text.strip()
        return value

    def prompt_choice(
        self,
        _description: str,
        options: list[str],
        default_option: str | None = None,
    ) -> str:
        value = self._next().strip()
        if value == "" and default_option is not None:
            return default_option
        allowed = {option.lower(): option for option in options}
        selected = allowed.get(value.lower())
        if selected is None:
            raise AssertionError(f"Invalid test answer: {value}")
        return selected

    def prompt_choice_with_justification(
        self,
        description: str,
        options: list[str],
        default_option: str | None = None,
        default_justification: str | None = None,
    ) -> tuple[str, str]:
        answer = self.prompt_choice(description, options, default_option=default_option)
        justification = self._next().strip()
        if justification == "" and default_justification is not None:
            return answer, default_justification
        return answer, justification

    def prompt_args_form(
        self,
        _args: list[AutomatedActionArg],
        initial_values: dict[str, str] | None = None,
    ) -> dict[str, str]:
        return {} if initial_values is None else cast(dict[str, str], dict(initial_values))

    def run_command(self, _command: str) -> int:
        return 0


class InteractiveRunnerFlowCliTest(unittest.TestCase):
    def test_workspace_root_prefers_bazel_workspace_directory(self) -> None:
        with mock.patch.dict(
            os.environ,
            {
                "BUILD_WORKSPACE_DIRECTORY": "/workspace/root",
                "BUILD_WORKING_DIRECTORY": "/workspace/other",
            },
            clear=False,
        ):
            self.assertEqual(_workspace_root(), "/workspace/root")

    def test_create_parser_supports_no_prefill_flag(self) -> None:
        parser = interactive_runner_cli._create_parser()
        args = parser.parse_args(["--no-prefill-from-last-run"])
        self.assertEqual(args.no_prefill_from_last_run, True)

    def test_run_analysis_prefills_from_previous_results_file(self) -> None:
        analysis_yaml = """
requirements:
  - REQ-TEST-001
steps:
  - action:
    description: Collect notes
  - assertion:
    description: Any errors?
    positive: No
    negative: Yes
"""
        with tempfile.TemporaryDirectory() as tmpdir, mock.patch.dict(
            os.environ, {"BUILD_WORKSPACE_DIRECTORY": tmpdir}
        ):
            analysis_path = Path(tmpdir) / "analysis.yaml"
            results_path = Path(tmpdir) / "results.json"
            analysis_path.write_text(analysis_yaml.strip() + "\n", encoding="utf-8")
            results_path.write_text(
                json.dumps(
                    {
                        "analysis": str(analysis_path),
                        "results": [
                            {
                                "type": "action",
                                "description": "Collect notes",
                                "result": "previous finding",
                            },
                            {
                                "type": "assertion",
                                "description": "Any errors?",
                                "answer": "No",
                                "passed": True,
                                "justification": "no problematic paths found",
                            },
                        ],
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

            steps, _ = load_analysis(analysis_path)
            prefill = _PrefillState.load(results_path)
            ui = _FakeUi(["\x01", "", ""])
            results = run_analysis(
                steps,
                cast("_RunnerUI", cast(object, ui)),
                analysis_path=analysis_path,
                results_path=results_path,
                prefill=prefill,
            )

            self.assertEqual(results[0]["result"], "previous finding")
            self.assertEqual(results[1]["answer"], "No")
            self.assertEqual(results[1]["passed"], True)
            self.assertEqual(
                results[1]["justification"], "no problematic paths found"
            )

    def test_run_analysis_prefills_repeat_until_from_legacy_iterations(self) -> None:
        analysis_yaml = """
requirements:
  - REQ-TEST-001
steps:
  - repeat:
    until:
      description: Additional repetition necessary?
      continue: Continue
      break: Stop
    steps:
      - action:
        description: Collect detail
  - assertion:
    description: Final status?
    positive: No
    negative: Yes
"""
        with tempfile.TemporaryDirectory() as tmpdir, mock.patch.dict(
            os.environ, {"BUILD_WORKSPACE_DIRECTORY": tmpdir}
        ):
            analysis_path = Path(tmpdir) / "analysis.yaml"
            results_path = Path(tmpdir) / "results.json"
            analysis_path.write_text(analysis_yaml.strip() + "\n", encoding="utf-8")
            results_path.write_text(
                json.dumps(
                    {
                        "analysis": str(analysis_path),
                        "results": [
                            {
                                "type": "repeat",
                                "until": "Additional repetition necessary?",
                                "iterations": [
                                    [
                                        {
                                            "type": "action",
                                            "description": "Collect detail",
                                            "result": "first",
                                        }
                                    ],
                                    [
                                        {
                                            "type": "action",
                                            "description": "Collect detail",
                                            "result": "second",
                                        }
                                    ],
                                ],
                            },
                            {
                                "type": "assertion",
                                "description": "Final status?",
                                "answer": "No",
                                "passed": True,
                            },
                        ],
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

            steps, _ = load_analysis(analysis_path)
            prefill = _PrefillState.load(results_path)
            ui = _FakeUi(["\x01", "", "\x01", "", "", ""])
            results = run_analysis(
                steps,
                cast("_RunnerUI", cast(object, ui)),
                analysis_path=analysis_path,
                results_path=results_path,
                prefill=prefill,
            )

            repeat_result = results[0]
            self.assertEqual(repeat_result["until_answers"], ["Continue", "Stop"])
            self.assertEqual(results[-1]["answer"], "No")

    def test_main_writes_results(self) -> None:
        analysis_yaml = """
requirements:
  - REQ-TEST-001
steps:
  - action:
    description: Collect notes
  - assertion:
    description: Any errors?
    positive: No
    negative: Yes
"""
        with tempfile.TemporaryDirectory() as tmpdir, mock.patch.dict(
            os.environ, {"BUILD_WORKSPACE_DIRECTORY": tmpdir}
        ):
            analysis_path = Path(tmpdir) / "analysis.yaml"
            results_path = Path(tmpdir) / "results.json"
            analysis_path.write_text(analysis_yaml.strip() + "\n", encoding="utf-8")

            with mock.patch.object(
                interactive_runner_cli,
                "_SplitPaneUI",
                return_value=_FakeUi(["note line 1", "No", "checked logs"]),
            ):
                interactive_runner_cli.main(
                    [
                        "--analysis",
                        str(analysis_path),
                        "--results-file",
                        str(results_path),
                    ]
                )

            payload = json.loads(results_path.read_text(encoding="utf-8"))
            self.assertEqual(payload["results"][0]["type"], "action")
            self.assertEqual(payload["results"][1]["passed"], True)
            self.assertEqual(payload["results"][1]["justification"], "checked logs")

    def test_run_analysis_writes_partial_results_with_failed_final_assertion(
        self,
    ) -> None:
        analysis_yaml = """
requirements:
  - REQ-TEST-001
steps:
  - action:
    description: Collect notes
  - assertion:
    description: Any errors?
    positive: No
    negative: Yes
"""
        with tempfile.TemporaryDirectory() as tmpdir, mock.patch.dict(
            os.environ, {"BUILD_WORKSPACE_DIRECTORY": tmpdir}
        ):
            analysis_path = Path(tmpdir) / "analysis.yaml"
            results_path = Path(tmpdir) / "results.json"
            analysis_path.write_text(analysis_yaml.strip() + "\n", encoding="utf-8")

            with self.assertRaises(KeyboardInterrupt):
                run_analysis(
                    load_analysis(analysis_path)[0],
                    cast("_RunnerUI", cast(object, _FakeUi(["note line"]))),
                    analysis_path=analysis_path,
                    results_path=results_path,
                )

            payload = json.loads(results_path.read_text(encoding="utf-8"))
            self.assertEqual(payload["results"][0]["type"], "action")
            self.assertEqual(payload["results"][-1]["type"], "assertion")
            self.assertEqual(payload["results"][-1]["passed"], False)
            self.assertEqual(payload["results"][-1]["partial_run"], True)

    def test_main_persists_failed_assertion_result(self) -> None:
        analysis_yaml = """
requirements:
  - REQ-TEST-001
steps:
  - assertion:
    description: Any errors?
    positive: No
    negative: Yes
"""
        with tempfile.TemporaryDirectory() as tmpdir, mock.patch.dict(
            os.environ, {"BUILD_WORKSPACE_DIRECTORY": tmpdir}
        ):
            analysis_path = Path(tmpdir) / "analysis.yaml"
            results_path = Path(tmpdir) / "results.json"
            analysis_path.write_text(analysis_yaml.strip() + "\n", encoding="utf-8")

            with mock.patch.object(
                interactive_runner_cli,
                "_SplitPaneUI",
                return_value=_FakeUi(["Yes", "observed terminate call"]),
            ):
                with self.assertRaises(SystemExit):
                    interactive_runner_cli.main(
                        [
                            "--analysis",
                            str(analysis_path),
                            "--results-file",
                            str(results_path),
                        ]
                    )

            payload = json.loads(results_path.read_text(encoding="utf-8"))
            self.assertEqual(payload["results"][-1]["type"], "assertion")
            self.assertEqual(payload["results"][-1]["passed"], False)
            self.assertEqual(
                payload["results"][-1]["justification"],
                "observed terminate call",
            )

    def test_main_exits_on_failed_assertion(self) -> None:
        analysis_yaml = """
requirements:
  - REQ-TEST-001
steps:
  - assertion:
    description: Any errors?
    positive: No
    negative: Yes
"""
        with tempfile.TemporaryDirectory() as tmpdir, mock.patch.dict(
            os.environ, {"BUILD_WORKSPACE_DIRECTORY": tmpdir}
        ):
            analysis_path = Path(tmpdir) / "analysis.yaml"
            results_path = Path(tmpdir) / "results.json"
            analysis_path.write_text(analysis_yaml.strip() + "\n", encoding="utf-8")

            with mock.patch.object(
                interactive_runner_cli,
                "_SplitPaneUI",
                return_value=_FakeUi(["Yes", "observed terminate call"]),
            ):
                with self.assertRaises(SystemExit) as cm:
                    interactive_runner_cli.main(
                        [
                            "--analysis",
                            str(analysis_path),
                            "--results-file",
                            str(results_path),
                        ]
                    )

            self.assertEqual(getattr(cm.exception, "code", None), 1)

    def test_main_exits_gracefully_on_interrupt(self) -> None:
        analysis_yaml = """
requirements:
  - REQ-TEST-001
steps:
  - assertion:
    description: Any errors?
    positive: No
    negative: Yes
"""
        with tempfile.TemporaryDirectory() as tmpdir, mock.patch.dict(
            os.environ, {"BUILD_WORKSPACE_DIRECTORY": tmpdir}
        ):
            analysis_path = Path(tmpdir) / "analysis.yaml"
            results_path = Path(tmpdir) / "results.json"
            analysis_path.write_text(analysis_yaml.strip() + "\n", encoding="utf-8")

            with mock.patch.object(
                interactive_runner_cli,
                "run_analysis",
                side_effect=KeyboardInterrupt,
            ):
                with self.assertRaises(SystemExit) as cm:
                    interactive_runner_cli.main(
                        [
                            "--analysis",
                            str(analysis_path),
                            "--results-file",
                            str(results_path),
                        ]
                    )

            self.assertEqual(getattr(cm.exception, "code", None), 130)

    def test_main_exits_gracefully_on_eof(self) -> None:
        analysis_yaml = """
requirements:
  - REQ-TEST-001
steps:
  - assertion:
    description: Any errors?
    positive: No
    negative: Yes
"""
        with tempfile.TemporaryDirectory() as tmpdir, mock.patch.dict(
            os.environ, {"BUILD_WORKSPACE_DIRECTORY": tmpdir}
        ):
            analysis_path = Path(tmpdir) / "analysis.yaml"
            results_path = Path(tmpdir) / "results.json"
            analysis_path.write_text(analysis_yaml.strip() + "\n", encoding="utf-8")

            with mock.patch.object(
                interactive_runner_cli,
                "run_analysis",
                side_effect=EOFError,
            ):
                with self.assertRaises(SystemExit) as cm:
                    interactive_runner_cli.main(
                        [
                            "--analysis",
                            str(analysis_path),
                            "--results-file",
                            str(results_path),
                        ]
                    )

            self.assertEqual(getattr(cm.exception, "code", None), 130)


if __name__ == "__main__":
    unittest.main()
