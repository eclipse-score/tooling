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


# Safety bound on the disambiguation suffix. Reaching it implies thousands of
# identically-named artefacts, which is a misconfiguration rather than a real
# data set; failing loudly beats an unbounded O(n^2) scan.
_MAX_UNIQUE_KEY_SUFFIX = 10000


def unique_key(existing: dict, base: str) -> str:
    """Return ``base`` (or ``base_2``, ``base_3``, …) not already in ``existing``."""
    key = base
    suffix = 2
    while key in existing:
        if suffix > _MAX_UNIQUE_KEY_SUFFIX:
            raise ValueError(
                f"Could not derive a unique key for {base!r} after "
                f"{_MAX_UNIQUE_KEY_SUFFIX} attempts — too many name collisions."
            )
        key = f"{base}_{suffix}"
        suffix += 1
    return key


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
