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

import signal
import unittest
from pathlib import Path
from unittest import mock

from manual_analysis import interactive_runner_runtime as runtime


class InteractiveRunnerRuntimeTest(unittest.TestCase):
    def test_signal_handler_tracks_last_signal_and_raises_interrupt(self) -> None:
        runtime._LAST_SIGNAL = None
        with self.assertRaises(KeyboardInterrupt):
            runtime._signal_handler(signal.SIGINT, None)
        self.assertEqual(runtime._LAST_SIGNAL, signal.SIGINT)

    def test_install_signal_handlers_registers_sigint_and_sigterm(self) -> None:
        with mock.patch.object(runtime.signal, "signal") as install:
            runtime._install_signal_handlers()
        install.assert_has_calls(
            [
                mock.call(signal.SIGINT, runtime._signal_handler),
                mock.call(signal.SIGTERM, runtime._signal_handler),
            ]
        )

    def test_interrupt_exit_code_depends_on_last_signal(self) -> None:
        runtime._LAST_SIGNAL = signal.SIGTERM
        self.assertEqual(runtime._interrupt_exit_code(), 143)
        runtime._LAST_SIGNAL = signal.SIGINT
        self.assertEqual(runtime._interrupt_exit_code(), 130)

    def test_workspace_root_prefers_environment_variable(self) -> None:
        with mock.patch.dict(
            runtime.os.environ,
            {"BUILD_WORKSPACE_DIRECTORY": "/workspace/root"},
            clear=False,
        ):
            self.assertEqual(runtime._workspace_root(), "/workspace/root")

    def test_workspace_root_falls_back_to_cwd_without_workspace_markers(self) -> None:
        with (
            mock.patch.dict(runtime.os.environ, {}, clear=True),
            mock.patch.object(runtime.Path, "exists", return_value=False),
            mock.patch.object(runtime.Path, "cwd", return_value=Path("/tmp/fallback")),
        ):
            self.assertEqual(runtime._workspace_root(), "/tmp/fallback")


if __name__ == "__main__":
    unittest.main()
