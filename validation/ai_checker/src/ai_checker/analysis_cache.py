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
Cache management for AI analysis results.

This module provides caching functionality for storing and retrieving
analysis results based on hash keys.
"""

import json
import logging
import os
from typing import Optional

from ai_checker.analysis_models import AnalysisResults

logger = logging.getLogger(__name__)


class AnalysisCache:
    """
    Manages caching of analysis results with hash-based key interface.
    """

    def __init__(self, cache_dir: Optional[str] = None):
        """
        Initialize the cache.

        Args:
            cache_dir: Optional directory path for caching results.
                      If None, caching is disabled.
        """
        self._cache_dir = cache_dir
        if self._cache_dir:
            os.makedirs(self._cache_dir, exist_ok=True)

    def get(self, cache_hash: str) -> Optional[AnalysisResults]:
        """
        Load cached analysis results.

        Args:
            cache_hash: SHA256 hash key for the analysis

        Returns:
            Cached AnalysisResults or None if not found
        """
        if not self._cache_dir:
            return None

        cache_file = os.path.join(self._cache_dir, f"{cache_hash}.json")
        if os.path.exists(cache_file):
            try:
                with open(cache_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                return AnalysisResults.model_validate(data)
            except Exception as exc:
                logger.warning(
                    "Failed to read cache file %s: %s: %s",
                    cache_file,
                    type(exc).__name__,
                    exc,
                )
                return None
        return None

    def set(self, cache_hash: str, results: AnalysisResults) -> None:
        """
        Save analysis results to cache.

        Args:
            cache_hash: SHA256 hash key for the analysis
            results: AnalysisResults to cache
        """
        if not self._cache_dir:
            return

        cache_file = os.path.join(self._cache_dir, f"{cache_hash}.json")
        try:
            with open(cache_file, "w", encoding="utf-8") as f:
                f.write(results.model_dump_json(indent=2))
        except Exception as exc:
            logger.warning(
                "Failed to write cache file %s: %s: %s",
                cache_file,
                type(exc).__name__,
                exc,
            )

    def is_enabled(self) -> bool:
        """
        Check if caching is enabled.

        Returns:
            True if cache directory is configured, False otherwise
        """
        return self._cache_dir is not None
