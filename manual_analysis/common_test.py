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

    def test_resolve_path_resolves_existing_relative_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            cwd = Path(tmpdir)
            file_path = cwd / "local.txt"
            file_path.write_text("x", encoding="utf-8")

            previous_cwd = Path.cwd()
            os.chdir(cwd)
            try:
                resolved = resolve_path("local.txt")
            finally:
                os.chdir(previous_cwd)

            self.assertEqual(resolved, file_path.resolve())

    def test_resolve_path_uses_build_working_directory_when_available(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            base = Path(tmpdir)
            nested = base / "sub" / "entry.txt"
            nested.parent.mkdir(parents=True, exist_ok=True)
            nested.write_text("x", encoding="utf-8")

            with mock.patch.dict(
                os.environ, {"BUILD_WORKING_DIRECTORY": str(base)}, clear=False
            ):
                resolved = resolve_path("sub/entry.txt")

            self.assertEqual(resolved, nested)

    def test_resolve_path_returns_unmodified_candidate_when_not_found(self) -> None:
        with mock.patch.dict(os.environ, {}, clear=True):
            self.assertEqual(resolve_path("missing/file.txt"), Path("missing/file.txt"))


if __name__ == "__main__":
    unittest.main()
