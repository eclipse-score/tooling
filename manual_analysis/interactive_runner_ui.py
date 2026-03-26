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
"""UI factory for the interactive manual-analysis runner."""

from __future__ import annotations

import sys
from typing import Callable

from manual_analysis.interactive_runner_runtime import _HAS_PROMPT_TOOLKIT
from manual_analysis.interactive_runner_ui_console import _ConsoleUI
from manual_analysis.interactive_runner_ui_split import _SplitPaneUI


def _make_ui(input_fn: Callable[[str], str] | None) -> _ConsoleUI | _SplitPaneUI:
    if input_fn is not None:
        return _ConsoleUI(input_fn)

    if _HAS_PROMPT_TOOLKIT and sys.stdin.isatty() and sys.stdout.isatty():
        try:
            import prompt_toolkit  # noqa: F401
            import wcwidth  # noqa: F401

            return _SplitPaneUI()
        except ModuleNotFoundError as err:
            print(
                "WARNING: prompt_toolkit TUI unavailable; falling back to console mode "
                f"({err})",
                file=sys.stderr,
            )

    return _ConsoleUI(input)
