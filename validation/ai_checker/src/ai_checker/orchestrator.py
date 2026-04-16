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
Orchestrator for artefact analysis workflow.

This module provides the main orchestration logic for extracting and analyzing
artefacts against engineering guidelines using pluggable extractors.
"""

import argparse
import asyncio
import concurrent.futures
import logging
import os
import sys
from typing import Optional

from langchain_core.language_models.chat_models import BaseChatModel

from ai_checker.ai_checker_core import AIChecker
from ai_checker.analysis_models import AnalysisResults
from ai_checker.requirement_extractor import RequirementExtractor
from ai_checker.result_formatter import ResultFormatter
from ai_checker.guidelines_reader import GuidelinesReader
from ai_checker.constants import DEFAULT_MODEL


def _create_default_chat_model(
    model_name: str = DEFAULT_MODEL,
    max_completion_tokens: int = 8192,
) -> BaseChatModel:
    """
    Create the default chat model using the GitHub Copilot SDK adapter.

    Uses the ChatCopilot LangChain wrapper as the default AI backend.

    Args:
        model_name: Model identifier (e.g. 'gpt-4.1', 'claude-sonnet-4')
        max_completion_tokens: Maximum tokens for completion

    Returns:
        Configured BaseChatModel instance (ChatCopilot)
    """
    from copilot_adapter.copilot_langchain import ChatCopilot

    return ChatCopilot(
        model=model_name,
        timeout=max(120.0, max_completion_tokens / 50.0),
    )


def _load_custom_ai_model_module(custom_path: str):
    """
    Load a custom ai_model module from a file path.

    The custom module must provide a `create_chat_model` function with
    the signature: `create_chat_model(model_name: str, max_completion_tokens: int) -> BaseChatModel`

    WARNING: This executes arbitrary Python code from the given path.  Only
    pass paths to files that you own and trust.  Never set --custom-ai-model
    to a path derived from untrusted external input.

    Args:
        custom_path: Path to custom ai_model.py file (must be a trusted file)

    Returns:
        The loaded custom ai_model module
    """
    logger = logging.getLogger(__name__)
    logger.info(f"--> Using custom ai_model from: {custom_path}")
    import importlib.util

    spec = importlib.util.spec_from_file_location("custom_ai_model", custom_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class AnalysisOrchestrator:
    """
    Main orchestrator class responsible for coordinating artefact
    extraction and analysis.
    """

    def __init__(
        self,
        model_name: str = DEFAULT_MODEL,
        guidelines_path: str = "guidelines",
        cache_dir: str | None = None,
        debug_log: str | None = None,
        batch_size: int | None = None,
        custom_ai_model: str | None = None,
        max_concurrent_requests: int = 5,
        max_batch_chars: int = 50000,
    ):
        """
        Initialize the orchestrator with AI checker.

        Args:
            model_name: Name of the AI model to use
            guidelines_path: Relative path to guidelines directory
            cache_dir: Optional directory path for caching results
            debug_log: Optional file path for detailed debug logging
            batch_size: Optional number of requirements to process per batch
            custom_ai_model: Optional path to custom ai_model.py file
            max_concurrent_requests: Maximum number of concurrent API requests
            max_batch_chars: Maximum total characters per batch
        """
        self.model_name = model_name
        self.guidelines_path = guidelines_path
        self._custom_ai_model = custom_ai_model

        # Initialize requirement extractor (no input directory yet)
        self.requirement_extractor = None

        # Load guidelines using GuidelinesReader
        self.guidelines_reader = GuidelinesReader(guidelines_path)
        all_guidelines = self.guidelines_reader.get_all_guidelines()
        self.guidelines_content = "\n\n".join(all_guidelines.values())

        # Create AI model (private member)
        self._chat_model: BaseChatModel = None
        if custom_ai_model and os.path.exists(custom_ai_model):
            # Use custom ai_model.py provided by the user
            ai_model_module = _load_custom_ai_model_module(custom_ai_model)
            self._chat_model = ai_model_module.create_chat_model(
                model_name=model_name,
                max_completion_tokens=8192,
            )
        else:
            # Default: use GitHub Copilot SDK via ChatCopilot adapter
            logger = logging.getLogger(__name__)
            logger.info("--> Using default ChatCopilot model adapter")
            self._chat_model = _create_default_chat_model(
                model_name=model_name,
                max_completion_tokens=8192,
            )

        # Initialize AI checker
        self.ai_checker = AIChecker(
            model_name=model_name,
            cache_dir=cache_dir,
            debug_log=debug_log,
            batch_size=batch_size,
            max_concurrent_requests=max_concurrent_requests,
            max_batch_chars=max_batch_chars,
        )

        # Initialize result formatter (will be configured with results later)
        self.result_formatter = None

        # Extractor instance (will be set when analyzing)
        self.artefact_extractor = None

        # Stored artefacts from extraction (reused for formatting)
        self._extracted_artefacts = None

    def analyze_directory(
        self, input_dir: str, dependency_dirs: list[str] | None = None
    ) -> AnalysisResults:
        """
        Extract and analyze artefacts from a directory using TRLC
        extractor.

        Args:
            input_dir: Path to directory containing files to analyze
            dependency_dirs: Optional list of directories containing
                dependencies for link resolution

        Returns:
            AnalysisResults containing structured analyses for each artefact
        """
        # Initialize TRLC requirement extractor
        self.artefact_extractor = RequirementExtractor(input_dir, dependency_dirs)

        # Extract artefacts
        artefacts = self.artefact_extractor.extract()
        self._extracted_artefacts = artefacts

        if not artefacts:
            print(
                f"WARNING: No artefacts found in '{input_dir}'. "
                "Architecture analysis is not yet implemented.",
                file=sys.stderr,
            )
            return AnalysisResults(analyses=[])

        # Analyze artefacts using AI checker with guidelines and chat model.
        # asyncio.run() will raise RuntimeError if there is already a running
        # event loop (e.g. inside pytest-asyncio or Jupyter).  In that case,
        # delegate to a fresh thread that owns its own event loop.
        coro = self.ai_checker.analyze(
            artefacts, self.guidelines_content, self._chat_model
        )
        try:
            asyncio.get_running_loop()
            # We're inside a running loop — run the coroutine in a new thread.
            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                analysis_results = pool.submit(asyncio.run, coro).result()
        except RuntimeError:
            # No running loop — safe to call asyncio.run() directly.
            analysis_results = asyncio.run(coro)

        return analysis_results

    def format_and_output(
        self,
        analysis_results: AnalysisResults,
        output_file: str = None,
        html_file: str = None,
        guidelines_output_dir: str = None,
    ) -> None:
        """Format and output analysis results.

        Args:
            analysis_results: AnalysisResults to format and output
            output_file: Output file for JSON results (None for stdout)
            html_file: Output file for HTML report (optional)
            guidelines_output_dir: Output directory for guideline pages (optional)
        """
        # Use previously extracted artefacts (avoids re-parsing)
        original_artefacts = self._extracted_artefacts

        # Initialize result formatter with analysis results
        self.result_formatter = ResultFormatter(
            analysis_results,
            model_name=self.model_name,
            guidelines_reader=self.guidelines_reader,
            guidelines_output_dir=guidelines_output_dir,
            original_requirements=original_artefacts,
        )

        # Output JSON results (primary output)
        if output_file:
            self.result_formatter.output(output_file)
        else:
            self.result_formatter.output(None)  # Print to stdout

        # Output HTML report if requested
        if html_file:
            self.result_formatter.output(html_file)


def argument_parser() -> argparse.ArgumentParser:
    """Create argument parser for CLI."""
    parser = argparse.ArgumentParser(
        description="Analyze TRLC requirements against engineering guidelines"
    )
    parser.add_argument(
        "-i",
        "--input",
        required=True,
        help="Path to directory containing TRLC files to analyze",
    )
    parser.add_argument(
        "--deps",
        action="append",
        default=[],
        help=(
            "Additional directories for dependency resolution "
            "(can be specified multiple times)"
        ),
    )
    parser.add_argument(
        "-o",
        "--output",
        default=None,
        help="Output file for JSON analysis results (required for Bazel rules)",
    )
    parser.add_argument(
        "--html",
        default=None,
        help="Output file for HTML report (optional)",
    )
    parser.add_argument(
        "-g",
        "--guidelines",
        default="guidelines",
        help="Relative path to guidelines directory (default: guidelines)",
    )
    parser.add_argument(
        "-m",
        "--model",
        default=DEFAULT_MODEL,
        help=f"AI model to use (default: {DEFAULT_MODEL})",
    )
    parser.add_argument(
        "-c",
        "--cache",
        default=None,
        help="Directory path for caching analysis results (optional)",
    )
    parser.add_argument(
        "--guidelines-output",
        default=None,
        help="Output directory for guideline HTML pages (optional)",
    )
    parser.add_argument(
        "-b",
        "--batch-size",
        type=int,
        default=None,
        help=(
            "Number of requirements to process per batch (optional, "
            "default: process all at once)"
        ),
    )
    parser.add_argument(
        "--max-concurrent-requests",
        type=int,
        default=5,
        help="Maximum number of concurrent API requests (default: 5)",
    )
    parser.add_argument(
        "--max-batch-chars",
        type=int,
        default=50000,
        help="Maximum total characters per batch (default: 50000)",
    )
    parser.add_argument(
        "--custom-ai-model",
        default=None,
        help="Path to custom ai_model.py file (optional)",
    )
    parser.add_argument(
        "--debug-log",
        default=None,
        help="Path to write debug output (stderr) for Bazel output_groups (optional)",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Enable verbose debug logging to debug log file (requires --debug-log)",
    )
    return parser


def main() -> None:
    """Main entry point for the orchestrator CLI."""
    parser = argument_parser()
    args = parser.parse_args()

    try:
        # Initialize orchestrator and analyze
        orchestrator = AnalysisOrchestrator(
            model_name=args.model,
            guidelines_path=args.guidelines,
            cache_dir=args.cache,
            debug_log=args.debug_log if args.verbose else None,
            batch_size=args.batch_size,
            custom_ai_model=args.custom_ai_model,
            max_concurrent_requests=args.max_concurrent_requests,
            max_batch_chars=args.max_batch_chars,
        )
        analysis_results = orchestrator.analyze_directory(args.input, args.deps)

        # Format and output results
        orchestrator.format_and_output(
            analysis_results,
            output_file=args.output,
            html_file=args.html,
            guidelines_output_dir=args.guidelines_output,
        )
    except Exception:
        # Let exceptions propagate with full traceback
        raise


if __name__ == "__main__":
    main()
