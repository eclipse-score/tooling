#!/usr/bin/env python3
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
"""Unit tests for the per-test coverage merger."""

import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from coverage.merger import find_llvm_profdata, get_object_files_from_manifest, is_elf


class IsElfTest(unittest.TestCase):
    def test_elf_magic_is_detected(self):
        with tempfile.NamedTemporaryFile(suffix=".bin") as f:
            f.write(b"\x7fELF" + b"\x00" * 12)
            f.flush()
            self.assertTrue(is_elf(Path(f.name)))

    def test_text_file_is_not_elf(self):
        with tempfile.NamedTemporaryFile(suffix=".txt") as f:
            f.write(b"just some text")
            f.flush()
            self.assertFalse(is_elf(Path(f.name)))

    def test_missing_file_is_not_elf(self):
        self.assertFalse(is_elf(Path("/nonexistent/path/binary")))


class FindLlvmProfdataTest(unittest.TestCase):
    def test_llvm_profdata_env_wins(self):
        with tempfile.NamedTemporaryFile() as f:
            with mock.patch.dict(os.environ, {"LLVM_PROFDATA": f.name}, clear=True):
                self.assertEqual(find_llvm_profdata(), f.name)

    def test_rust_llvm_profdata_resolved_against_root(self):
        with tempfile.TemporaryDirectory() as root:
            tool = Path(root) / "bin" / "llvm-profdata"
            tool.parent.mkdir()
            tool.write_bytes(b"\x7fELF")
            env = {"RUST_LLVM_PROFDATA": "bin/llvm-profdata", "ROOT": root}
            with mock.patch.dict(os.environ, env, clear=True):
                self.assertEqual(find_llvm_profdata(), str(tool))

    def test_returns_empty_when_nothing_resolves(self):
        env = {"RUST_LLVM_PROFDATA": "does/not/exist"}
        with mock.patch.dict(os.environ, env, clear=True):
            self.assertEqual(find_llvm_profdata(), "")


class GetObjectFilesFromManifestTest(unittest.TestCase):
    def test_missing_root_env_is_a_hard_error(self):
        """Without ROOT the merger cannot resolve manifest paths — must exit."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt") as manifest:
            manifest.write("some/path\n")
            manifest.flush()
            with mock.patch.dict(os.environ, {}, clear=True):
                with self.assertRaises(SystemExit):
                    get_object_files_from_manifest(Path(manifest.name))

    def test_rust_elf_manifest_entry_is_collected(self):
        """rules_rust lists the instrumented test executable in the manifest."""
        with tempfile.TemporaryDirectory() as root:
            binary = Path(root) / "pkg" / "my_test"
            binary.parent.mkdir()
            binary.write_bytes(b"\x7fELF" + b"\x00" * 12)
            with tempfile.NamedTemporaryFile(mode="w", suffix=".txt") as manifest:
                manifest.write("pkg/my_test\n")
                manifest.flush()
                with mock.patch.dict(os.environ, {"ROOT": root}, clear=True):
                    objects = get_object_files_from_manifest(Path(manifest.name))
        self.assertEqual(objects, {str(binary)})

    def test_external_manifest_entries_are_skipped(self):
        """Toolchain-provided metadata files (external/) are not instrumented objects."""
        with tempfile.TemporaryDirectory() as root:
            binary = Path(root) / "external" / "tool"
            binary.parent.mkdir()
            binary.write_bytes(b"\x7fELF" + b"\x00" * 12)
            with tempfile.NamedTemporaryFile(mode="w", suffix=".txt") as manifest:
                manifest.write("external/tool\n")
                manifest.flush()
                with mock.patch.dict(os.environ, {"ROOT": root}, clear=True):
                    objects = get_object_files_from_manifest(Path(manifest.name))
        self.assertEqual(objects, set())

    def test_objects_list_entries_are_resolved(self):
        """C++ tests provide an objects_list.txt with one object path per line."""
        with tempfile.TemporaryDirectory() as root:
            obj = Path(root) / "bazel-out" / "lib.a"
            obj.parent.mkdir()
            obj.write_bytes(b"!<arch>\n")
            objects_list = Path(root) / "objects_list.txt"
            objects_list.write_text("bazel-out/lib.a\n")
            with tempfile.NamedTemporaryFile(mode="w", suffix=".txt") as manifest:
                manifest.write(f"{objects_list}\n")
                manifest.flush()
                with mock.patch.dict(os.environ, {"ROOT": root}, clear=True):
                    objects = get_object_files_from_manifest(Path(manifest.name))
        self.assertEqual(objects, {str(obj)})


if __name__ == "__main__":
    unittest.main()
