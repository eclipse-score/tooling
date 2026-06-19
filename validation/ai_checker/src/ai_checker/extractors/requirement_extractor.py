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
Extracts structured requirement data from TRLC files.

This module provides functionality to parse TRLC requirement files and extract
requirement metadata into a structured format suitable for AI analysis.
"""

import logging
import os
from typing import Any

import trlc.ast
from trlc.errors import Message_Handler
from trlc.trlc import Source_Manager

from ai_checker.extractors.base import ArtefactExtractor, unique_key

logger = logging.getLogger(__name__)


class RequirementExtractor(ArtefactExtractor):
    """Extracts structured requirement data from TRLC files."""

    def __init__(
        self,
        input_directory: str | None = None,
        dependency_directories: list[str] | None = None,
        req_files: list[str] | None = None,
    ):
        """
        Initialize the RequirementExtractor with directory paths.

        Args:
            input_directory: Optional path to a directory containing TRLC
                files to analyze. When ``req_files`` is given this is not
                required: the explicit files then define both what is parsed
                and the grading scope.
            dependency_directories: Optional list of additional
                directories for link resolution
            req_files: Optional list of individual TRLC files to register
                instead of scanning the entire input directory. When set,
                only these files are registered (so other files present in
                the same directory are not picked up by TRLC) and exactly
                these files form the grading scope.
        """
        # Use realpath (not abspath) so the scope check in
        # extract_requirements_data() is symlink-safe: a symlinked source file
        # is compared on its resolved path against the resolved scope.
        self.input_directory = (
            os.path.realpath(input_directory) if input_directory else None
        )
        self.dependency_directories = [
            os.path.realpath(d) for d in (dependency_directories or [])
        ]
        self.req_files = [os.path.realpath(f) for f in (req_files or [])]
        self.symbols: trlc.ast.Symbol_Table | None = None

    def parse_trlc_files(self) -> trlc.ast.Symbol_Table:
        """
        Parse TRLC files in the specified directories.

        When ``req_files`` was supplied at construction time, only those
        individual files are registered from the input directory; dependency
        directories are still registered in full for link resolution.
        When ``req_files`` is empty (the default), all directories including
        the input directory are registered (original behaviour).

        Returns:
            Symbol table containing parsed TRLC objects

        Raises:
            ValueError: If parsing fails
        """
        message_handler = Message_Handler()
        source_manager = Source_Manager(message_handler)

        if self.req_files:
            # Register only the specific req files declared in the Bazel target.
            # This avoids picking up extra .trlc files in the same directory that
            # are not part of the target and may fail TRLC validation.
            for file_path in self.req_files:
                source_manager.register_file(file_path)

            # Register dependency directories in full for cross-reference / link
            # resolution (these dirs are controlled by Bazel deps and are expected
            # to be valid).
            for dep_dir in self.dependency_directories:
                source_manager.register_directory(dep_dir)
        else:
            # Original behaviour: register all directories (input + deps).
            # Collect all directories and filter out overlapping ones.
            all_dirs = (
                [self.input_directory] if self.input_directory else []
            ) + self.dependency_directories

            # Remove duplicates and filter out directories that are
            # subdirectories of others
            unique_dirs = []
            for dir_path in sorted(set(all_dirs)):
                # Check if this directory is a subdirectory of any already
                # registered directory
                is_subdir = False
                for existing_dir in unique_dirs:
                    if dir_path.startswith(existing_dir + os.sep):
                        is_subdir = True
                        break

                # Also check if any existing directory is a subdirectory of this one
                # In that case, remove the existing one and add this one
                dirs_to_remove = []
                for i, existing_dir in enumerate(unique_dirs):
                    if existing_dir.startswith(dir_path + os.sep):
                        dirs_to_remove.append(i)

                for i in reversed(dirs_to_remove):
                    unique_dirs.pop(i)

                if not is_subdir:
                    unique_dirs.append(dir_path)

            # Register all unique, non-overlapping directories
            for dir_path in unique_dirs:
                source_manager.register_directory(dir_path)

        symbols = source_manager.process()
        if symbols is None:
            raise ValueError("Failed to parse TRLC Files")

        self.symbols = symbols
        return symbols

    def extract_field_value(
        self, obj: trlc.ast.Record_Object, field_name: str
    ) -> Any | None:
        """
        Extract a field value from a TRLC Record_Object.

        This function handles multiple field types:
        - Implicit_Null: Returns None for null/empty fields
        - Record_Reference: Resolves reference objects to their target's
          fully qualified name by accessing the 'target' attribute and
          calling fully_qualified_name() on it
        - String values: Returns the value from the 'value' attribute
        - Other types: Returns the field object as-is

        Args:
            obj: The TRLC Record_Object to extract from
            field_name: Name of the field to extract

        Returns:
            The extracted field value (string, FQN for references, None
            for null fields), or None if the field does not exist
        """
        try:
            # Try to get field from the record object's members
            if hasattr(obj, "field") and field_name in obj.field:
                field = obj.field[field_name]

                # Handle Implicit_Null objects (null values)
                if isinstance(field, trlc.ast.Implicit_Null):
                    return None

                # Handle Record_Reference objects (for parent requirements)
                if isinstance(field, trlc.ast.Record_Reference):
                    if hasattr(field, "target") and field.target is not None:
                        return field.target.fully_qualified_name()
                    return None

                # Handle field with value attribute (strings, etc.)
                if hasattr(field, "value"):
                    return field.value

                return field
            return None
        except (AttributeError, KeyError):
            return None

    def extract_requirements_data(self) -> list[dict[str, Any]]:
        """
        Extract structured requirement data from TRLC symbol table.

        The grading scope is, in order of precedence, the explicit ``req_files``
        (directory-independent), then the ``input_directory`` prefix, then all
        parsed objects. Objects outside the scope (e.g. dependencies) are used
        for link resolution only and are not graded.

        Returns:
            List of dictionaries, each containing:
            - unique_id: Fully qualified requirement name
            - description: Requirement description text
            - parent_requirement: Parent requirement ID if present
            - requirement_type: Type of the requirement
        """
        if self.symbols is None:
            self.parse_trlc_files()

        requirements = []

        # Determine the grading scope. When explicit req files are given they
        # define the scope exactly (directory-independent, so requirements
        # spread across several directories are all graded). Otherwise fall
        # back to the input directory prefix, and if neither is set grade every
        # parsed object.
        req_file_scope = set(self.req_files)

        for obj in self.symbols.iter_record_objects():
            obj_file_path = os.path.realpath(obj.location.file_name)
            if req_file_scope:
                if obj_file_path not in req_file_scope:
                    continue
            elif self.input_directory is not None:
                # Use `+ os.sep` to avoid false-positive prefix matches
                # (e.g. /foo/bar matching /foo/barbaz).
                if not obj_file_path.startswith(self.input_directory + os.sep):
                    continue

            unique_id = obj.fully_qualified_name()

            # Extract description field
            description = self.extract_field_value(obj, "description")
            if description is None:
                description = ""

            # Extract parent requirement field
            parent_requirement = self.extract_field_value(obj, "parent")

            # Get requirement type
            requirement_type = (
                obj.n_typ.name
                if hasattr(obj, "n_typ") and hasattr(obj.n_typ, "name")
                else "Unknown"
            )

            requirements.append(
                {
                    "unique_id": unique_id,
                    "description": str(description),
                    "parent_requirement": str(parent_requirement)
                    if parent_requirement
                    else None,
                    "requirement_type": requirement_type,
                }
            )

        return requirements

    def extract(self) -> dict[str, dict[str, Any]]:
        """
        Parse TRLC files and extract structured requirement data.

        Returns:
            Dictionary mapping requirement IDs to their metadata:
            {
                "requirement_id": {
                    "description": "...",
                    "parent": "...",
                    "type": "..."
                }
            }
        """
        self.parse_trlc_files()
        requirements_list = self.extract_requirements_data()

        # Convert list format to dictionary format for the interface
        artefacts = {}
        for req in requirements_list:
            req_id = req["unique_id"]

            # Guard against any object that wasn't fully resolved to a string
            # (extract_field_value returns None or str for Record_Reference,
            # but be defensive against future TRLC API changes).
            parent = req["parent_requirement"]
            if parent is not None and not isinstance(parent, str):
                parent = "[not resolved]"

            # Guard against duplicate fully-qualified names: disambiguate with a
            # suffix instead of letting a later object silently overwrite (and
            # drop) an earlier requirement from the graded set.
            key = unique_key(artefacts, req_id)
            if key != req_id:
                logger.warning(
                    "Duplicate requirement ID %r — grading the duplicate as %r.",
                    req_id,
                    key,
                )

            artefacts[key] = {
                "description": req["description"],
                "parent": parent,
                "type": req["requirement_type"],
            }

        return artefacts
