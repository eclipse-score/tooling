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
AI Checker for analyzing requirements against guidelines.

This module provides the core AI analysis functionality for requirements.
"""

import asyncio
import hashlib
import json
import logging
import sys
import time
from typing import Any

from langchain_core.language_models.chat_models import BaseChatModel
from langchain_core.messages import HumanMessage, SystemMessage
from pydantic import ValidationError

from ai_checker.analysis_cache import AnalysisCache
from ai_checker.analysis_models import AnalysisResults
from ai_checker.constants import DEFAULT_MODEL


class AIChecker:
    """
    Analyzes requirements against engineering guidelines
    using an AI model with structured output.
    """

    def __init__(
        self,
        model_name: str = DEFAULT_MODEL,
        cache_dir: str | None = None,
        debug_log: str | None = None,
        batch_size: int | None = None,
        max_concurrent_requests: int = 5,
        max_batch_chars: int = 50000,
    ):
        """
        Initialize the AI Checker with model configuration.

        Args:
            model_name: Name of the AI model to use (for cache key
                generation)
            cache_dir: Optional directory path for caching results. If
                None, caching is disabled.
            debug_log: Optional file path for detailed debug logging.
                If provided, verbose debug output is written to this file.
            batch_size: Optional number of requirements to process per
                batch. If None, process all at once.
            max_concurrent_requests: Maximum number of concurrent API requests
            max_batch_chars: Maximum total characters per batch
        """
        self._model_name = model_name
        self._cache = AnalysisCache(cache_dir)
        self._batch_size = batch_size
        self._max_concurrent_requests = max_concurrent_requests
        self._max_batch_chars = max_batch_chars
        self._semaphore = asyncio.Semaphore(max_concurrent_requests)

        # Set up logger (use fixed name to prevent handler leaks across instances)
        self._logger = logging.getLogger(f"{__name__}.AIChecker")
        self._logger.setLevel(logging.DEBUG)
        self._logger.propagate = False
        self._logger.handlers.clear()

        # Always add stderr handler for INFO and above (progress messages)
        stderr_handler = logging.StreamHandler(sys.stderr)
        stderr_handler.setLevel(logging.INFO)
        stderr_formatter = logging.Formatter("%(message)s")
        stderr_handler.setFormatter(stderr_formatter)
        self._logger.addHandler(stderr_handler)

        # Add file handler for DEBUG level if debug log requested
        if debug_log:
            file_handler = logging.FileHandler(debug_log, mode="w")
            file_handler.setLevel(logging.DEBUG)
            file_formatter = logging.Formatter("%(message)s")
            file_handler.setFormatter(file_formatter)
            self._logger.addHandler(file_handler)

    def _generate_cache_key(
        self, artefacts: dict[str, dict[str, Any]], guidelines_content: str
    ) -> str:
        """
        Generate a unique cache key for the given artefacts.

        Args:
            artefacts: Dictionary mapping artefact IDs to their metadata
            guidelines_content: Combined content of all guidelines

        Returns:
            SHA256 hash of the artefacts, guidelines content, and model name
        """
        # Create a deterministic string representation of artefacts
        artefact_data = json.dumps(artefacts, sort_keys=True)
        combined = f"{artefact_data}:{guidelines_content}:{self._model_name}"
        return hashlib.sha256(combined.encode("utf-8")).hexdigest()

    def _format_artefacts_for_analysis(
        self, artefacts: dict[str, dict[str, Any]]
    ) -> str:
        """
        Format extracted artefacts for AI analysis.

        Args:
            artefacts: Dictionary mapping artefact IDs to their metadata

        Returns:
            Formatted string representation of artefacts
        """
        formatted = "Requirements to analyze:\n\n"
        for artefact_id, metadata in artefacts.items():
            formatted += f"ID: {artefact_id}\n"

            # Format all metadata fields
            for key, value in metadata.items():
                if value:
                    formatted += f"{key.capitalize()}: {value}\n"

            formatted += "\n"

        return formatted

    def _create_batches(
        self, artefacts: dict[str, dict[str, Any]]
    ) -> list[dict[str, dict[str, Any]]]:
        """
        Create batches based on both count and total character size.

        Args:
            artefacts: Dictionary mapping artefact IDs to their metadata

        Returns:
            List of batches, where each batch is a dict of artefacts
        """
        batches = []
        current_batch = {}
        current_char_count = 0

        for artefact_id, metadata in artefacts.items():
            # Calculate character count for this artefact
            artefact_text = self._format_artefacts_for_analysis({artefact_id: metadata})
            char_count = len(artefact_text)

            # Check if adding this artefact would exceed limits
            would_exceed_count = (
                self._batch_size and len(current_batch) >= self._batch_size
            )
            would_exceed_chars = current_char_count + char_count > self._max_batch_chars

            # Start new batch if necessary
            if current_batch and (would_exceed_count or would_exceed_chars):
                batches.append(current_batch)
                current_batch = {}
                current_char_count = 0

            # Add artefact to current batch
            current_batch[artefact_id] = metadata
            current_char_count += char_count

        # Add final batch if not empty
        if current_batch:
            batches.append(current_batch)

        return batches

    async def analyze(
        self,
        artefacts: dict[str, dict[str, Any]],
        guidelines_content: str,
        chat_model: BaseChatModel,
    ) -> AnalysisResults:
        """
        Analyze artefacts using the chat model with structured output.
        Uses async processing with rate limiting for concurrent requests.
        Uses caching if enabled to avoid redundant API calls.

        Args:
            artefacts: Dictionary mapping artefact IDs to their metadata
            guidelines_content: Combined content of all guidelines
            chat_model: BaseChatModel instance for AI analysis

        Returns:
            AnalysisResults containing structured analyses for each artefact
        """
        # Log number of artefacts to be analyzed
        num_artefacts = len(artefacts)
        self._logger.info(f"--> Analyzing {num_artefacts} requirement(s)...")

        # Create batches based on count and character size
        batches = self._create_batches(artefacts)
        num_batches = len(batches)

        if num_batches > 1:
            self._logger.info(
                f"--> Created {num_batches} batches based on size and count limits"
            )
            self._logger.info(
                f"--> Processing {num_batches} batches concurrently "
                f"(max {self._max_concurrent_requests} at a time)"
            )

        total_start_time = time.time()

        # Create tasks for all batches to process concurrently
        batch_tasks = [
            self._analyze_batch_async(i + 1, batch, guidelines_content, chat_model)
            for i, batch in enumerate(batches)
        ]

        # Execute all batch tasks concurrently with rate limiting
        # Use return_exceptions=True to continue even if some batches fail
        all_batch_results = await asyncio.gather(*batch_tasks, return_exceptions=True)

        # Flatten results from all batches, handling exceptions
        all_analyses = []
        failed_batches = 0
        for i, batch_results in enumerate(all_batch_results):
            if isinstance(batch_results, Exception):
                failed_batches += 1
                self._logger.warning(
                    f"--> WARNING: Batch {i + 1} failed with error: "
                    f"{type(batch_results).__name__}: {str(batch_results)}"
                )
            else:
                all_analyses.extend(batch_results)

        if failed_batches > 0:
            self._logger.warning(
                f"--> WARNING: {failed_batches} out of {num_batches} batches failed. "
                f"Successfully analyzed {len(all_analyses)} requirement(s)."
            )

        # Calculate final statistics
        current_total_cost = 0.0
        if chat_model and hasattr(chat_model, "total_costs"):
            current_total_cost = getattr(chat_model, "total_costs", 0.0)

        # Log final statistics
        total_elapsed = time.time() - total_start_time
        all_scores = [a.score for a in all_analyses]
        average_score = sum(all_scores) / len(all_scores) if all_scores else 0.0

        self._logger.info(f"--> Execution time: {total_elapsed:.2f}s")
        self._logger.info(f"--> Total costs: ${current_total_cost:.4f} USD")
        self._logger.info(f"--> Overall average score: {average_score:.2f}")

        return AnalysisResults(analyses=all_analyses)

    async def _analyze_batch_async(
        self,
        batch_number: int,
        artefacts: dict[str, dict[str, Any]],
        guidelines_content: str,
        chat_model: BaseChatModel,
    ) -> list[Any]:
        """
        Analyze a batch of artefacts using async with rate limiting.

        Args:
            batch_number: The batch number (1-indexed) for logging
            artefacts: Dictionary mapping artefact IDs to their metadata
            guidelines_content: Combined content of all guidelines
            chat_model: BaseChatModel instance for AI analysis

        Returns:
            List of analysis results for all artefacts in the batch
        """
        # Log batch start
        batch_size = len(artefacts)
        self._logger.info(
            f"--> Batch {batch_number}: Processing {batch_size} requirement(s)..."
        )

        self._logger.debug(
            f"Batch {batch_number} contains artefact IDs: {', '.join(artefacts.keys())}"
        )
        self._logger.debug(
            f"Guidelines content length: {len(guidelines_content)} characters"
        )

        # Check cache first
        cache_hash = self._generate_cache_key(artefacts, guidelines_content)
        cached_result = self._cache.get(cache_hash)
        if cached_result is not None:
            self._logger.info(f"--> Batch {batch_number}: Completed (from cache)")
            return cached_result.analyses

        # Use semaphore for rate limiting
        async with self._semaphore:
            try:
                self._logger.debug(
                    f"Batch {batch_number}: Creating structured chat model..."
                )

                # Create structured chat model
                structured_chat = chat_model.with_structured_output(AnalysisResults)

                # Prepare system message with guidelines
                system_message = SystemMessage(content=guidelines_content)

                # Format requirements for analysis
                formatted_artefacts = self._format_artefacts_for_analysis(artefacts)

                self._logger.debug(
                    f"Batch {batch_number}: Formatted artefacts length: "
                    f"{len(formatted_artefacts)} characters"
                )
                self._logger.debug(
                    f"Batch {batch_number}: ===== RAW AI MODEL INPUT ====="
                )
                self._logger.debug(
                    f"Batch {batch_number}: System Message (Guidelines):"
                )
                self._logger.debug(guidelines_content)
                self._logger.debug(f"Batch {batch_number}: ---")
                self._logger.debug(
                    f"Batch {batch_number}: Human Message (Requirements):"
                )
                self._logger.debug(formatted_artefacts)
                self._logger.debug(
                    f"Batch {batch_number}: ===== END RAW AI MODEL INPUT ====="
                )
                self._logger.debug(
                    f"Batch {batch_number}: Sending request to AI model "
                    f"({self._model_name})..."
                )

                analysis_prompt = HumanMessage(content=formatted_artefacts)

                # Call async invoke
                start_time = time.time()
                response = await structured_chat.ainvoke(
                    [system_message, analysis_prompt]
                )
                elapsed = time.time() - start_time

                self._logger.debug(
                    f"Batch {batch_number}: Received response in {elapsed:.2f}s"
                )

                # Validate that we got a proper response
                if not hasattr(response, "analyses") or not response.analyses:
                    raise ValueError(
                        f"AI model returned empty or invalid response. "
                        f"Expected 'analyses' field with {len(artefacts)} "
                        f"items, got: {response}"
                    )

                # Cache the result
                self._cache.set(cache_hash, response)

                # Log batch completion
                self._logger.info(f"--> Batch {batch_number}: Completed successfully")

                return response.analyses

            except ValidationError as e:
                self._logger.error(
                    f"\n\n--> Batch {batch_number}: AI Model Error "
                    f"(returned invalid response):"
                )
                self._logger.error("--> Validation errors:")
                for error in e.errors():
                    field = error.get("loc", ["unknown"])[0]
                    msg = error.get("msg", "Unknown error")
                    input_val = error.get("input", "N/A")
                    self._logger.error(f"    - Field '{field}': {msg}")
                    if input_val != "N/A":
                        self._logger.error(
                            f"      Received: {json.dumps(input_val, indent=6)}"
                        )
                raise
            except Exception as e:
                self._logger.error(
                    f"--> Batch {batch_number}: AI Model Error: "
                    f"{type(e).__name__}: {str(e)}"
                )
                raise
