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
"""Runtime helpers for the interactive manual-analysis runner."""

from __future__ import annotations

import importlib.util
import os
import signal
from pathlib import Path

_HAS_PROMPT_TOOLKIT = importlib.util.find_spec("prompt_toolkit") is not None
_LAST_SIGNAL: int | None = None


def _signal_handler(signum: int, _frame) -> None:  # type: ignore[no-untyped-def]
    global _LAST_SIGNAL
    _LAST_SIGNAL = signum
    raise KeyboardInterrupt


def _install_signal_handlers() -> None:
    signal.signal(signal.SIGINT, _signal_handler)
    signal.signal(signal.SIGTERM, _signal_handler)


def _interrupt_exit_code() -> int:
    if _LAST_SIGNAL == signal.SIGTERM:
        return 143
    return 130


def _workspace_root() -> str:
    workspace_dir = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if workspace_dir:
        return workspace_dir

    for parent in Path(__file__).resolve().parents:
        if (parent / "MODULE.bazel").exists() or (parent / "WORKSPACE").exists():
            return str(parent)

    return str(Path.cwd())
