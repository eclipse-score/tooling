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
"""Unit tests for sphinx_module_ext's confdir-aware needs loading."""

import json
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from sphinx_module_ext import init_external_needs


class TestInitExternalNeeds(unittest.TestCase):
    """Tests for init_external_needs's "config-inited" listener."""

    def test_uses_app_confdir_not_cwd(self) -> None:
        """Regression test: init_external_needs is invoked by Sphinx after its
        own chdir(confdir) (scoped to evaluating conf.py) has already been
        undone, so it must resolve needs_external_needs.json via app.confdir
        rather than the process's current working directory."""
        with tempfile.TemporaryDirectory() as tmp:
            confdir = Path(tmp) / "confdir"
            confdir.mkdir()
            (confdir / "needs_external_needs.json").write_text(
                json.dumps({"dep": {"json_path": "x", "version": "1.0"}}),
                encoding="utf-8",
            )
            elsewhere = Path(tmp) / "elsewhere"
            elsewhere.mkdir()
            old_cwd = Path.cwd()
            os.chdir(elsewhere)
            try:
                app = SimpleNamespace(confdir=str(confdir))
                config = SimpleNamespace()

                init_external_needs(app, config)
            finally:
                os.chdir(old_cwd)

            self.assertEqual(len(config.needs_external_needs), 1)


if __name__ == "__main__":
    unittest.main()
