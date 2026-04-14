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
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional, List, Set
import json
import subprocess


@dataclass
class CmdArgs:
    output: Optional[str] = None
    tags: List[str] = field(default_factory=list)
    namespace: Optional[str] = None
    inputs: List[str] = field(default_factory=list)

    def as_list(self) -> List[str]:
        """Returns the command line arguments as a list for lobster-bazel"""
        cmd_args: List[str] = []

        if self.output is not None:
            cmd_args.append(f"--output={self.output}")
        for tag in self.tags:
            cmd_args.extend(["--tag", tag])
        if self.namespace is not None:
            cmd_args.append(f"--namespace={self.namespace}")
        cmd_args.extend(self.inputs)

        return cmd_args


class TestRunner:
    def __init__(self, working_dir: Path, tool_path: Path):
        self.working_dir = working_dir
        self.tool_path = tool_path
        self.cmd_args = CmdArgs()

    def get_tool_args(self) -> List[str]:
        """Returns the command line arguments for 'lobster-bazel'"""
        return self.cmd_args.as_list()

    def run_tool_test(self):
        """Runs the tool with the specified command line arguments and returns
        the completed process."""

        cmd = [str(self.tool_path)] + self.get_tool_args()
        completed_process = subprocess.run(
            cmd,
            cwd=self.working_dir,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        return completed_process

    def copy_file_to_working_directory(self, source: Path):
        """Copies a file from the source path to the working directory."""
        destination = self.working_dir / source.name
        destination.write_bytes(source.read_bytes())

    def _read_lobster_file(self, lobster_file: Path) -> dict:
        try:
            content = json.loads(lobster_file.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            raise ValueError(f"Failed to parse {lobster_file} as JSON: {e}") from e

        if "data" not in content:
            raise ValueError(
                f"{lobster_file} invalid .lobster file: missing 'data' key"
            )
        return content

    def verify_lobster_tags(
        self, lobster_file: Path, expected_tags: List[str]
    ) -> Set[str]:
        """
        Parse a .lobster file and verify it contains all expected tag names.
        """

        content = self._read_lobster_file(lobster_file)

        found_tags: Set[str] = {
            item["tag"] for item in content["data"] if "tag" in item
        }
        return set(expected_tags) - found_tags

    def extract_lobster_file_names(self, lobster_file: Path) -> Set[str]:
        """
        Parse a .lobster file and verify it contains all expected file names.
        """

        content = self._read_lobster_file(lobster_file)

        found_file_names: Set[str] = {
            Path(item["location"]["file"]).name for item in content["data"]
        }
        return found_file_names
