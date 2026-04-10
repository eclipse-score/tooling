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

import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from manual_analysis.common import resolve_path


class CommonTest(unittest.TestCase):
    def test_resolve_path_returns_existing_absolute_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            file_path = Path(tmpdir) / "data.txt"
            file_path.write_text("x", encoding="utf-8")
            self.assertEqual(resolve_path(str(file_path)), file_path)

    def test_resolve_path_uses_bazel_runfiles_when_available(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            runfile = Path(tmpdir) / "tool.txt"
            runfile.write_text("x", encoding="utf-8")
            runfiles = mock.Mock()
            runfiles.Rlocation.return_value = str(runfile)

            with mock.patch(
                "manual_analysis.common._create_runfiles", return_value=runfiles
            ):
                resolved = resolve_path("_main/manual_analysis/tool.txt")

            self.assertEqual(resolved, runfile)
            runfiles.Rlocation.assert_called_once_with("_main/manual_analysis/tool.txt")

    def test_resolve_path_falls_back_when_runfiles_lookup_fails(self) -> None:
        runfiles = mock.Mock()
        runfiles.Rlocation.return_value = None

        with mock.patch(
            "manual_analysis.common._create_runfiles", return_value=runfiles
        ):
            resolved = resolve_path("missing/in/runfiles.txt")

        self.assertEqual(resolved, Path("missing/in/runfiles.txt"))


if __name__ == "__main__":
    unittest.main()
