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
"""
Reader for guidelines markdown files.

This module provides functionality to read and manage guidelines
from a directory of markdown files.
"""

import logging
import os

logger = logging.getLogger(__name__)


class GuidelinesReader:
    """Reader for guidelines markdown files."""

    def __init__(self, guidelines_dir: str):
        """
        Initialize the GuidelinesReader and load all guidelines.

        Args:
            guidelines_dir: Path to guidelines directory containing
                markdown files.
        """
        self.guidelines_dir = guidelines_dir

        # Dictionary to store all guideline contents keyed by filename
        # (without extension)
        self.guidelines: dict[str, str] = {}

        # Load all markdown files from the directory
        self._load_all_guidelines()

    def _load_all_guidelines(self):
        """Load all markdown files from the guidelines directory."""
        if not os.path.isdir(self.guidelines_dir):
            logger.warning(f"Guidelines directory not found: {self.guidelines_dir}")
            return

        for filename in sorted(os.listdir(self.guidelines_dir)):
            if filename.endswith(".md"):
                file_path = os.path.join(self.guidelines_dir, filename)
                # Use filename without extension as key
                key = os.path.splitext(filename)[0]
                content = self._read_file(file_path)
                if content:
                    self.guidelines[key] = content

    def _read_file(self, file_path: str) -> str:
        """Read a file and return its content as a string.

        Args:
            file_path: Path to the file to read

        Returns:
            File content as string, or empty string if file not found
        """
        try:
            with open(file_path, encoding="utf-8") as f:
                return f.read()
        except FileNotFoundError:
            logger.warning(f"File not found: {file_path}")
            return ""
        except OSError as e:
            logger.warning(f"Error reading file {file_path}: {e}")
            return ""
        except UnicodeDecodeError as e:
            logger.warning(f"Unicode decode error reading file {file_path}: {e}")
            return ""

    def get_guideline(self, name: str) -> str:
        """Get a specific guideline by name.

        Args:
            name: Name of the guideline file (without .md extension)

        Returns:
            Guideline content as string, or empty string if not found
        """
        return self.guidelines.get(name, "")

    def get_all_guidelines(self) -> dict[str, str]:
        """Get all guidelines as a dictionary.

        Returns:
            Dictionary mapping guideline names to their content
        """
        return self.guidelines.copy()
