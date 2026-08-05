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
"""Tests for the sphinx_wrapper.py thin shim.

Persistent-worker mode itself (rules_python's Worker class/JSON protocol) is
not re-tested here -- it's loaded directly from rules_python at runtime, not
ported, so it's covered by rules_python's own test suite. These tests only
cover what actually lives in this file: the hermetic-env fixup, @file
expansion, and main()'s dispatch between one-shot and worker mode.
"""

import logging
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import sphinx_wrapper


class TestFixupHermeticToolEnv(unittest.TestCase):
    def setUp(self) -> None:
        self._saved = {var: os.environ.get(var) for var in sphinx_wrapper._HERMETIC_TOOL_ENV_VARS}

    def tearDown(self) -> None:
        for var, value in self._saved.items():
            if value is None:
                os.environ.pop(var, None)
            else:
                os.environ[var] = value

    def test_converts_relative_paths_to_absolute(self) -> None:
        os.environ["GRAPHVIZ_DOT"] = "third_party/docs_runtime/dot"
        sphinx_wrapper.fixup_hermetic_tool_env()
        self.assertTrue(os.path.isabs(os.environ["GRAPHVIZ_DOT"]))
        self.assertTrue(os.environ["GRAPHVIZ_DOT"].endswith("third_party/docs_runtime/dot"))

    def test_leaves_absolute_paths_unchanged(self) -> None:
        os.environ["PLANTUML_BIN"] = "/abs/path/to/plantuml"
        sphinx_wrapper.fixup_hermetic_tool_env()
        self.assertEqual(os.environ["PLANTUML_BIN"], "/abs/path/to/plantuml")

    def test_ignores_unset_vars(self) -> None:
        os.environ.pop("FTA_METAMODEL_DIR", None)
        sphinx_wrapper.fixup_hermetic_tool_env()  # must not raise
        self.assertNotIn("FTA_METAMODEL_DIR", os.environ)


class TestExpandParamFiles(unittest.TestCase):
    def test_expands_at_file_skipping_blank_lines(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            param_file = Path(tmp) / "args.params"
            param_file.write_text("src\nout\n\n-b\nhtml\n", encoding="utf-8")
            self.assertEqual(
                sphinx_wrapper.expand_param_files([f"@{param_file}"]),
                ["src", "out", "-b", "html"],
            )

    def test_passes_through_plain_args_unchanged(self) -> None:
        argv = ["src", "out", "-b", "html"]
        self.assertEqual(sphinx_wrapper.expand_param_files(argv), argv)


class TestMainDispatch(unittest.TestCase):
    def setUp(self) -> None:
        patcher = mock.patch.object(sphinx_wrapper, "fixup_hermetic_tool_env")
        self.addCleanup(patcher.stop)
        patcher.start()

    def test_dispatches_to_persistent_worker(self) -> None:
        with (
            mock.patch.object(sphinx_wrapper, "run_persistent_worker", return_value=0) as worker,
            mock.patch.object(sphinx_wrapper, "run_one_shot") as one_shot,
        ):
            exit_code = sphinx_wrapper.main(["src", "out", "--persistent_worker"])

        worker.assert_called_once()
        one_shot.assert_not_called()
        self.assertEqual(exit_code, 0)

    def test_dispatches_to_one_shot_with_expanded_argv(self) -> None:
        argv = ["src", "out", "-b", "html"]
        with (
            mock.patch.object(sphinx_wrapper, "run_one_shot", return_value=0) as one_shot,
            mock.patch.object(sphinx_wrapper, "run_persistent_worker") as worker,
        ):
            exit_code = sphinx_wrapper.main(argv)

        one_shot.assert_called_once_with(argv)
        worker.assert_not_called()
        self.assertEqual(exit_code, 0)


class TestInferWrapperLogLevel(unittest.TestCase):
    def test_defaults_to_info(self) -> None:
        self.assertEqual(sphinx_wrapper._infer_wrapper_log_level(["src", "out"]), logging.INFO)

    def test_quiet_flag_maps_to_warning(self) -> None:
        self.assertEqual(sphinx_wrapper._infer_wrapper_log_level(["src", "out", "-q"]), logging.WARNING)

    def test_vv_flag_maps_to_debug(self) -> None:
        self.assertEqual(sphinx_wrapper._infer_wrapper_log_level(["src", "out", "-vv"]), logging.DEBUG)

    def test_vvv_flag_maps_to_debug(self) -> None:
        self.assertEqual(sphinx_wrapper._infer_wrapper_log_level(["src", "out", "-vvv"]), logging.DEBUG)


class TestRunOneShot(unittest.TestCase):
    def setUp(self) -> None:
        self._saved_level = logging.getLogger().level
        self.addCleanup(logging.getLogger().setLevel, self._saved_level)

    def test_returns_sphinx_exit_code(self) -> None:
        with mock.patch.object(sphinx_wrapper, "sphinx_main", return_value=0):
            self.assertEqual(sphinx_wrapper.run_one_shot(["src", "out"]), 0)

    def test_returns_1_on_exception(self) -> None:
        with mock.patch.object(sphinx_wrapper, "sphinx_main", side_effect=RuntimeError("boom")):
            self.assertEqual(sphinx_wrapper.run_one_shot(["src", "out"]), 1)

    def test_applies_inferred_log_level(self) -> None:
        with mock.patch.object(sphinx_wrapper, "sphinx_main", return_value=0):
            sphinx_wrapper.run_one_shot(["src", "out", "-vv"])
        self.assertEqual(logging.getLogger().level, logging.DEBUG)


if __name__ == "__main__":
    unittest.main()
