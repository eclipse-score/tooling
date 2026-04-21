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
Abstract base class for artefact extractors.

This module defines the interface for extracting artefacts from various sources
for AI analysis.
"""

from abc import ABC, abstractmethod
from typing import Dict, Any


class ArtefactExtractor(ABC):
    """
    Abstract base class for extracting artefacts for AI analysis.

    Implementations should extract artefacts from their respective sources
    (e.g., TRLC requirements, code documentation, test cases) and return
    them in a standardized dictionary format.
    """

    @abstractmethod
    def extract(self) -> Dict[str, Dict[str, Any]]:
        """
        Extract artefacts and return them in a standardized format.

        Returns:
            Dictionary mapping artefact IDs to their metadata:
            {
                "artefact_id": {
                    "field1": "value1",
                    "field2": "value2",
                    ...
                }
            }
        """
        pass
