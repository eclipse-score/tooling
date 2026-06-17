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
Extracts architecture artefacts (PlantUML diagrams) for AI analysis.

Architecture designs are reviewed from their raw PlantUML source text. The AI
reads the ``.puml`` content directly, so this extractor simply reads each
diagram file and exposes its contents under the standardized artefact format.
"""

import logging
import os
from typing import Any

from ai_checker.extractors.base import ArtefactExtractor, unique_key

logger = logging.getLogger(__name__)


class ArchitectureExtractor(ArtefactExtractor):
    """Extracts raw PlantUML diagram source for AI analysis."""

    def __init__(self, puml_files: list[str]):
        """
        Initialize the ArchitectureExtractor with diagram file paths.

        Args:
            puml_files: List of paths to PlantUML (.puml) source files.
        """
        self.puml_files = [os.path.abspath(f) for f in puml_files]

    def extract(self) -> dict[str, dict[str, Any]]:
        """
        Read each PlantUML file and return its source as an artefact.

        Returns:
            Dictionary mapping diagram names to their metadata:
            {
                "<diagram-name>": {
                    "type": "plantuml",
                    "content": "<raw puml source>"
                }
            }
        """
        artefacts: dict[str, dict[str, Any]] = {}
        for file_path in self.puml_files:
            name = os.path.splitext(os.path.basename(file_path))[0]
            try:
                with open(file_path, encoding="utf-8") as f:
                    content = f.read()
            except OSError as exc:
                logger.warning(
                    "Skipping unreadable PlantUML file %s: %s", file_path, exc
                )
                continue
            if not content.strip():
                continue

            artefacts[unique_key(artefacts, name)] = {
                "type": "plantuml",
                "content": content,
            }

        return artefacts
