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
Reader for guidelines and background-context documents.

This module provides a single reader for text documents used by the AI
checker. It serves two callers that need the same behaviour:

* **Guidelines** — markdown files loaded from a directory (the graded rules).
* **Background context** — markdown and PlantUML files supplied as an explicit
  file list (read-only reference material).

Both are just "read text files into a name -> content mapping", so they share
one implementation rather than duplicating a near-identical reader.
"""

import logging
import os

from ai_checker.extractors.base import unique_key

logger = logging.getLogger(__name__)


class GuidelinesReader:
    """Reader for guidelines / context documents.

    Source can be either a directory (scanned for matching files) or an
    explicit list of file paths. Files are filtered by extension.
    """

    def __init__(
        self,
        guidelines_dir: str | None = None,
        *,
        files: list[str] | None = None,
        extensions: tuple[str, ...] = (".md",),
    ):
        """
        Initialize the reader and load all matching documents.

        Args:
            guidelines_dir: Path to a directory to scan (mutually exclusive
                with ``files``). Backwards-compatible positional argument.
            files: Explicit list of file paths to read.
            extensions: Tuple of accepted file extensions (lower-case, with
                leading dot).
        """
        self.guidelines_dir = guidelines_dir
        self._extensions = extensions

        # Mapping of document name (filename without extension) -> content.
        self.guidelines: dict[str, str] = {}

        if files is not None:
            self._load_files(files)
        elif guidelines_dir is not None:
            self._load_directory(guidelines_dir)

    def _matches(self, filename: str) -> bool:
        return filename.lower().endswith(self._extensions)

    def _add(self, file_path: str) -> None:
        """Read a file and register it under a unique name key."""
        content = self._read_file(file_path)
        if not content:
            return
        base = os.path.splitext(os.path.basename(file_path))[0]
        self.guidelines[unique_key(self.guidelines, base)] = content

    def _load_directory(self, directory: str) -> None:
        """Load all matching files from a directory."""
        if not os.path.isdir(directory):
            logger.warning(f"Guidelines directory not found: {directory}")
            return
        for filename in sorted(os.listdir(directory)):
            if self._matches(filename):
                self._add(os.path.join(directory, filename))

    def _load_files(self, files: list[str]) -> None:
        """Load an explicit list of files, filtered by extension."""
        for file_path in files:
            if self._matches(file_path):
                self._add(file_path)

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
        """Get a specific document by name.

        Args:
            name: Name of the file (without extension)

        Returns:
            Document content as string, or empty string if not found
        """
        return self.guidelines.get(name, "")

    def get_all_guidelines(self) -> dict[str, str]:
        """Get all documents as a dictionary.

        Returns:
            Dictionary mapping document names to their content
        """
        return self.guidelines.copy()

    def get_combined(self) -> str:
        """Return all document contents concatenated, in name order."""
        return "\n\n".join(self.guidelines[k] for k in sorted(self.guidelines))
