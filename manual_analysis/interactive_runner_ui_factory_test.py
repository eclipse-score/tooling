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

import builtins
import unittest
from types import SimpleNamespace
from unittest import mock

from manual_analysis import interactive_runner_ui as ui_factory
from manual_analysis.interactive_runner_ui_console import _ConsoleUI
from manual_analysis.interactive_runner_ui_split import _SplitPaneUI


class InteractiveRunnerUiFactoryTest(unittest.TestCase):
    def test_make_ui_uses_console_when_input_fn_is_provided(self) -> None:
        ui = ui_factory._make_ui(lambda _: "")
        self.assertIsInstance(ui, _ConsoleUI)

    def test_make_ui_uses_console_when_prompt_toolkit_flag_is_false(self) -> None:
        with mock.patch.object(ui_factory, "_HAS_PROMPT_TOOLKIT", False):
            ui = ui_factory._make_ui(None)
        self.assertIsInstance(ui, _ConsoleUI)

    def test_make_ui_uses_split_pane_when_tty_and_deps_available(self) -> None:
        with (
            mock.patch.object(ui_factory, "_HAS_PROMPT_TOOLKIT", True),
            mock.patch.object(
                ui_factory.sys, "stdin", SimpleNamespace(isatty=lambda: True)
            ),
            mock.patch.object(
                ui_factory.sys, "stdout", SimpleNamespace(isatty=lambda: True)
            ),
            mock.patch.dict(
                "sys.modules",
                {"prompt_toolkit": mock.Mock(), "wcwidth": mock.Mock()},
                clear=False,
            ),
        ):
            ui = ui_factory._make_ui(None)
        self.assertIsInstance(ui, _SplitPaneUI)

    def test_make_ui_falls_back_to_console_when_wcwidth_import_fails(self) -> None:
        original_import = builtins.__import__

        def _import(name, *args, **kwargs):  # type: ignore[no-untyped-def]
            if name == "wcwidth":
                raise ModuleNotFoundError("wcwidth")
            return original_import(name, *args, **kwargs)

        with (
            mock.patch.object(ui_factory, "_HAS_PROMPT_TOOLKIT", True),
            mock.patch.object(
                ui_factory.sys, "stdin", SimpleNamespace(isatty=lambda: True)
            ),
            mock.patch.object(
                ui_factory.sys, "stdout", SimpleNamespace(isatty=lambda: True)
            ),
            mock.patch("builtins.__import__", side_effect=_import),
            mock.patch("builtins.print") as print_mock,
        ):
            ui = ui_factory._make_ui(None)

        self.assertIsInstance(ui, _ConsoleUI)
        print_mock.assert_called_once()


if __name__ == "__main__":
    unittest.main()
