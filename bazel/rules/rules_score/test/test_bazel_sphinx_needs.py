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
"""Unit tests for bazel_sphinx_needs.load_external_needs()'s base_dir handling."""

import json
import os
import tempfile
import unittest
from pathlib import Path

from bazel_sphinx_needs import load_external_needs


def _write_needs_file(directory: Path) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "needs_external_needs.json").write_text(
        json.dumps({"dep": {"json_path": "bazel-out/dep/needs.json", "version": "1.0"}}),
        encoding="utf-8",
    )


class TestLoadExternalNeeds(unittest.TestCase):
    """Tests for load_external_needs's base_dir handling."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self._old_cwd = Path.cwd()

    def tearDown(self) -> None:
        os.chdir(self._old_cwd)
        self._tmp.cleanup()

    def test_explicit_base_dir_finds_file_regardless_of_cwd(self) -> None:
        """Regression test for the confdir-vs-cwd bug: a "config-inited"-style
        caller running with cwd != confdir must still find the file when it
        passes confdir explicitly instead of relying on the default cwd."""
        confdir = self.root / "confdir"
        _write_needs_file(confdir)
        elsewhere = self.root / "elsewhere"
        elsewhere.mkdir()
        os.chdir(elsewhere)

        result = load_external_needs(confdir)

        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["version"], "1.0")

    def test_default_base_dir_falls_back_to_cwd(self) -> None:
        """A conf.py module-level caller (no base_dir) relies on cwd == confdir,
        true while Sphinx's eval_config_file() chdir is active."""
        _write_needs_file(self.root)
        os.chdir(self.root)

        result = load_external_needs()

        self.assertEqual(len(result), 1)

    def test_missing_file_returns_empty_list(self) -> None:
        result = load_external_needs(self.root)

        self.assertEqual(result, [])


if __name__ == "__main__":
    unittest.main()
