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
"""Console-based UI for interactive manual analyses."""

from __future__ import annotations

import pydoc
import subprocess
from typing import Callable

from manual_analysis.interactive_runner_runtime import _workspace_root
from manual_analysis.yaml_schema import AutomatedActionArg


class _ConsoleUI:
    def __init__(self, input_fn: Callable[[str], str]) -> None:
        self._input_fn = input_fn

    def print_header(self, title: str) -> None:
        print("\n" + "=" * 80)
        print(title)
        print("=" * 80)

    def show_text(self, title: str, content: str) -> None:
        text = f"{title}\n\n{content}"
        if content.count("\n") > 25 or len(content) > 1600:
            pydoc.pager(text)
            return
        print(text)

    def prompt_choice(
        self,
        description: str,
        options: list[str],
        default_option: str | None = None,
    ) -> str:
        allowed = {value.lower(): value for value in options}
        while True:
            answer = self._input_fn(f"{description} [{'/'.join(options)}]: ").strip()
            if answer == "" and default_option is not None:
                return default_option
            selected = allowed.get(answer.lower())
            if selected:
                return selected
            print(f"Invalid answer '{answer}'. Expected one of: {', '.join(options)}")

    def prompt_justification(
        self,
        prompt: str,
        default_text: str | None = None,
    ) -> str:
        suffix = ""
        if default_text:
            suffix = f" [{default_text}]"
        answer = self._input_fn(f"{prompt}{suffix} (optional): ").strip()
        if answer == "" and default_text is not None:
            return default_text
        return answer

    def prompt_choice_with_justification(
        self,
        description: str,
        options: list[str],
        default_option: str | None = None,
        default_justification: str | None = None,
    ) -> tuple[str, str]:
        answer = self.prompt_choice(description, options, default_option=default_option)
        justification = self.prompt_justification(
            "Justification",
            default_text=default_justification,
        )
        return answer, justification

    def prompt_multiline(self, prompt: str, initial_text: str = "") -> str:
        print(prompt)
        print("Enter result text. Finish input with Ctrl+A (ASCII 0x01) on a new line.")
        lines: list[str] = []
        if initial_text:
            lines.append(initial_text)
        while True:
            line = self._input_fn("")
            if line == "\x01":
                return "\n".join(lines).strip()
            lines.append(line)

    def prompt_args_form(
        self,
        args: list[AutomatedActionArg],
        initial_values: dict[str, str] | None = None,
    ) -> dict[str, str]:
        values: dict[str, str] = {}
        prefill = initial_values or {}
        for index, arg in enumerate(args, start=1):
            default_text = prefill.get(arg.name)
            if default_text is None:
                default_text = "" if arg.default is None else arg.default
            hint = (
                f"Arg {index}/{len(args)}: {arg.name}"
                if arg.default is not None
                else f"Arg {index}/{len(args)}: {arg.name} (no default)"
            )
            print(hint)
            answer = self._input_fn(f"{arg.name} [{default_text}]: ")
            values[arg.name] = answer
        return values

    def run_command(self, command: str) -> int:
        process = subprocess.Popen(
            command,
            shell=True,
            cwd=_workspace_root(),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
        if process.stdout is not None:
            for line in process.stdout:
                print(line, end="")
        return process.wait()
