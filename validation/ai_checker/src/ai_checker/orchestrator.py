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

from ai_checker.ai_checker_core import AIChecker
from ai_checker.analysis_agent import AnalysisAgent
from ai_checker.analysis_models import AnalysisResults
from ai_checker.extractors.architecture_extractor import ArchitectureExtractor
from ai_checker.extractors.requirement_extractor import RequirementExtractor
from ai_checker.reports.formatter import ResultFormatter
from ai_checker.guidelines_reader import GuidelinesReader
from ai_checker.constants import DEFAULT_MODEL

# Request timeout heuristic: allow ~1 second of wall-clock per this many
# completion tokens, with a fixed floor. Generous because the Copilot CLI
# round-trip dominates for small requests.
_TOKENS_PER_TIMEOUT_SECOND = 50.0
_MIN_REQUEST_TIMEOUT_SECONDS = 120.0


def _create_default_agent(
    model_name: str = DEFAULT_MODEL,
    max_completion_tokens: int = 8192,
) -> AnalysisAgent:
    """
    Create the default analysis agent backed directly by the Copilot SDK.

    Args:
        model_name: Model identifier (e.g. 'gpt-4.1', 'claude-sonnet-4')
        max_completion_tokens: Maximum tokens for completion (used to size the
            request timeout)

    Returns:
        Configured AnalysisAgent instance (CopilotAgent)
    """
    from ai_checker.agents.copilot_agent import CopilotAgent

    return CopilotAgent(
        model=model_name,
        timeout=max(
            _MIN_REQUEST_TIMEOUT_SECONDS,
            max_completion_tokens / _TOKENS_PER_TIMEOUT_SECOND,
        ),
    )


def _agent_from_custom_module(module, model_name: str) -> AnalysisAgent:
    """Build an AnalysisAgent from a custom ai_model module.

    The module must expose ``create_agent(model_name) -> AnalysisAgent``. To use
    a LangChain model, the function can return
    ``LangChainAgent(SomeBaseChatModel(...))``.
    """
    if not hasattr(module, "create_agent"):
        raise AttributeError(
            "Custom ai_model module must define "
            "create_agent(model_name) -> AnalysisAgent"
        )
    return module.create_agent(model_name=model_name)


def _load_custom_ai_model_module(custom_path: str):
    """
    Load a custom ai_model module from a file path.

    The custom module must provide a `create_agent` function with the signature:
    `create_agent(model_name: str) -> AnalysisAgent`

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
    if spec is None or spec.loader is None:
        raise ImportError(
            f"Could not load a Python module from --custom-ai-model path: {custom_path}"
        )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class AnalysisOrchestrator:
    """
    Main orchestrator class responsible for coordinating artefact
    extraction and analysis.
    """

    # TODO(SRP): This class currently owns too many responsibilities — system
    # prompt assembly (general + project + context layering), agent
    # construction/loading, the event-loop-vs-thread execution strategy in
    # analyze_directory(), and report formatting/output. Split into focused
    # collaborators (prompt builder, agent factory, async runner, output
    # writer) once a unit-test suite exists to make the refactor safe. Tracked
    # for a follow-up PR.

    def __init__(
        self,
        model_name: str = DEFAULT_MODEL,
        guidelines_path: str = "guidelines",
        guideline_files: list[str] | None = None,
        cache_dir: str | None = None,
        debug_log: str | None = None,
        batch_size: int | None = None,
        custom_ai_model: str | None = None,
        max_concurrent_requests: int = 5,
        max_batch_chars: int = 50000,
        context_files: list[str] | None = None,
        project_guideline_files: list[str] | None = None,
    ):
        """
        Initialize the orchestrator with AI checker.

        Args:
            model_name: Name of the AI model to use
            guidelines_path: Relative path to a guidelines directory (scanned
                non-recursively). Used only when ``guideline_files`` is empty.
            guideline_files: Optional explicit list of guideline files (.md).
                Preferred over ``guidelines_path`` because guideline sets can
                span multiple directories; passing the files directly avoids
                relying on a single directory scan.
            cache_dir: Optional directory path for caching results
            debug_log: Optional file path for detailed debug logging
            batch_size: Optional number of requirements to process per batch
            custom_ai_model: Optional path to custom ai_model.py file
            max_concurrent_requests: Maximum number of concurrent API requests
            max_batch_chars: Maximum total characters per batch
            context_files: Optional list of background-context files (.md/.puml)
                injected into the system prompt as read-only reference material.
            project_guideline_files: Optional list of project-specific guideline
                files (.md) injected into the system prompt as graded rules,
                layered on top of the general and type guidelines.
        """
        self.model_name = model_name
        self.guidelines_path = guidelines_path
        self._custom_ai_model = custom_ai_model

        # Initialize requirement extractor (no input directory yet)
        self.requirement_extractor = None

        # Load guidelines using GuidelinesReader. Prefer an explicit file list
        # (handles guideline sets that span multiple directories) and fall back
        # to scanning a single directory for direct CLI use.
        if guideline_files:
            self.guidelines_reader = GuidelinesReader(files=guideline_files)
        else:
            self.guidelines_reader = GuidelinesReader(guidelines_path)
        self.guidelines_content = self.guidelines_reader.get_combined()

        self.system_prompt = self.guidelines_content

        # Layer project-specific guidelines (graded) on top of the general and
        # type guidelines. These carry project details such as requirement or
        # architecture levels and are evaluated like any other guideline.
        if project_guideline_files:
            project_reader = GuidelinesReader(files=project_guideline_files)
            project_content = project_reader.get_combined()
            if project_content:
                self.system_prompt += (
                    "\n\n# PROJECT-SPECIFIC GUIDELINES\n\n" + project_content
                )

        # Load optional background context (markdown + plantuml) and append it
        # as a clearly labelled, read-only section of the system prompt.
        if context_files:
            context_reader = GuidelinesReader(
                files=context_files, extensions=(".md", ".puml")
            )
            context_content = context_reader.get_combined()
            if context_content:
                self.system_prompt += (
                    "\n\n# BACKGROUND CONTEXT (reference only — not graded)\n\n"
                    + context_content
                )

        # Create the analysis agent (private member)
        logger = logging.getLogger(__name__)
        if custom_ai_model:
            # SECURITY: _load_custom_ai_model_module() executes arbitrary Python
            # from this path. The path comes from the trusted --custom-ai-model
            # flag / Bazel target; never wire it to untrusted external input. A
            # set-but-missing path is a hard error rather than a silent fallback
            # to the default agent, so a typo cannot mask the intended backend.
            if not os.path.exists(custom_ai_model):
                raise FileNotFoundError(
                    f"--custom-ai-model path does not exist: {custom_ai_model}"
                )
            ai_model_module = _load_custom_ai_model_module(custom_ai_model)
            self._agent: AnalysisAgent = _agent_from_custom_module(
                ai_model_module, model_name
            )
        else:
            # Default: use the GitHub Copilot SDK directly via CopilotAgent
            logger.info("--> Using default CopilotAgent (Copilot SDK)")
            self._agent = _create_default_agent(
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

        # Guard: agent is closed after the first analyze_directory() call.
        self._agent_closed: bool = False

    def analyze_directory(
        self,
        input_dir: str | None = None,
        dependency_dirs: list[str] | None = None,
        req_files: list[str] | None = None,
        artefact_type: str = "requirements",
        puml_files: list[str] | None = None,
    ) -> AnalysisResults:
        """
        Extract and analyze artefacts using the extractor for ``artefact_type``.

        Args:
            input_dir: Path to directory containing files to analyze
            dependency_dirs: Optional list of directories containing
                dependencies for link resolution (requirements only)
            req_files: Optional list of individual TRLC files to register
                instead of scanning the entire input directory (requirements
                only). When set, only those files are parsed.
            artefact_type: Either ``"requirements"`` (TRLC) or
                ``"architecture"`` (raw PlantUML).
            puml_files: List of PlantUML files to analyze (architecture only).

        Returns:
            AnalysisResults containing structured analyses for each artefact
        """
        if self._agent_closed:
            raise RuntimeError(
                "AnalysisOrchestrator.analyze_directory() may only be called once. "
                "Create a new orchestrator instance for each analysis run."
            )

        # Remember the artefact type for report metadata.
        self._artefact_type = artefact_type

        # Select the extractor for the requested artefact type.
        if artefact_type == "architecture":
            self.artefact_extractor = ArchitectureExtractor(puml_files or [])
        else:
            self.artefact_extractor = RequirementExtractor(
                input_dir,
                dependency_dirs,
                req_files=req_files or [],
            )

        # Extract artefacts
        artefacts = self.artefact_extractor.extract()
        self._extracted_artefacts = artefacts

        if not artefacts:
            logging.getLogger(__name__).warning(
                "No '%s' artefacts found in '%s'.",
                artefact_type,
                input_dir or "<req-files>",
            )
            return AnalysisResults(analyses=[])

        # Analyze artefacts using AI checker with the assembled system prompt
        # and agent.  Close the agent afterward so the CLI subprocess is shut
        # down in the same event loop that started it.
        # asyncio.run() will raise RuntimeError if there is already a running
        # event loop (e.g. inside pytest-asyncio or Jupyter).  In that case,
        # delegate to a fresh thread that owns its own event loop.
        agent = self._agent

        async def _analyze_and_close() -> AnalysisResults:
            try:
                return await self.ai_checker.analyze(
                    artefacts, self.system_prompt, agent
                )
            finally:
                # aclose() is part of the AnalysisAgent interface (no-op by
                # default), so cleanup is guaranteed for every backend.
                await agent.aclose()
                self._agent_closed = True

        coro = _analyze_and_close()
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
        output_file: str | None = None,
        html_file: str | None = None,
        guidelines_output_dir: str | None = None,
        rst_file: str | None = None,
    ) -> None:
        """Format and output analysis results.

        Builds one report in memory and renders each requested format directly
        from it.

        Args:
            analysis_results: AnalysisResults to format and output
            output_file: Output file for JSON results (None for stdout)
            html_file: Output file for HTML report (optional)
            guidelines_output_dir: Output directory for guideline pages (optional)
            rst_file: Output file for reStructuredText report (optional)
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
            artefact_type=getattr(self, "_artefact_type", "requirements"),
        )

        # Output JSON results (primary output)
        if output_file:
            self.result_formatter.output(output_file)
        else:
            self.result_formatter.output(None)  # Print to stdout

        # Output HTML report if requested
        if html_file:
            self.result_formatter.output(html_file)

        # Output reStructuredText report if requested
        if rst_file:
            self.result_formatter.output(rst_file)


def argument_parser() -> argparse.ArgumentParser:
    """Create argument parser for CLI."""
    parser = argparse.ArgumentParser(
        description="Analyze TRLC requirements against engineering guidelines"
    )
    parser.add_argument(
        "--req-file",
        action="append",
        default=[],
        dest="req_file",
        help=(
            "Individual TRLC file to register for analysis "
            "(can be specified multiple times). When provided, only these "
            "files are registered from the input directory instead of "
            "scanning the entire directory."
        ),
    )
    parser.add_argument(
        "-i",
        "--input",
        default=None,
        help=(
            "Path to directory containing artefact files to analyze. Optional "
            "when --req-file is used (the req files then define the grading "
            "scope) or for architecture review (which uses --puml-file)."
        ),
    )
    parser.add_argument(
        "--artefact-type",
        choices=["requirements", "architecture"],
        default="requirements",
        dest="artefact_type",
        help=(
            "Type of artefacts to analyze: 'requirements' (TRLC) or "
            "'architecture' (raw PlantUML). Default: requirements."
        ),
    )
    parser.add_argument(
        "--puml-file",
        action="append",
        default=[],
        dest="puml_file",
        help=(
            "Individual PlantUML file to analyze for architecture review "
            "(can be specified multiple times). Used with "
            "--artefact-type architecture."
        ),
    )
    parser.add_argument(
        "--context-file",
        action="append",
        default=[],
        dest="context_file",
        help=(
            "Background-context file (.md or .puml) injected into the system "
            "prompt as read-only reference material (can be specified multiple "
            "times)."
        ),
    )
    parser.add_argument(
        "--project-guidelines",
        action="append",
        default=[],
        dest="project_guidelines",
        help=(
            "Project-specific guideline file (.md) injected into the system "
            "prompt as a graded rule, layered on top of the general and type "
            "guidelines (can be specified multiple times)."
        ),
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
        "--rst",
        default=None,
        help="Output file for reStructuredText report (optional)",
    )
    parser.add_argument(
        "-g",
        "--guidelines",
        default="guidelines",
        help="Relative path to guidelines directory (default: guidelines)",
    )
    parser.add_argument(
        "--guidelines-file",
        action="append",
        default=[],
        dest="guidelines_file",
        help=(
            "Explicit guideline file (.md) to load (can be specified multiple "
            "times). Preferred over --guidelines: guideline sets can span "
            "several directories, and the Bazel rules pass every guideline "
            "file directly so none are dropped by a single directory scan."
        ),
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
    parser.add_argument(
        "--score-threshold",
        type=float,
        default=None,
        dest="score_threshold",
        help=(
            "Minimum average score (0-10) required to succeed. When set, the "
            "process exits non-zero if the average score is below this value. "
            "Used by the Bazel test rules."
        ),
    )
    return parser


def _report_location(output_path: str | None) -> str | None:
    """Return a human-friendly location for the generated reports.

    Inside a Bazel test, Bazel copies the undeclared outputs to
    ``bazel-testlogs/<package>/<name>/test.outputs/`` — a stable path relative
    to the workspace root. Derive it from ``$TEST_TARGET`` (``//package:name``)
    so the message points at the committed tree rather than the sandbox temp
    directory. Outside a test, fall back to the report's own directory.
    """
    test_target = os.environ.get("TEST_TARGET")
    if test_target and test_target.startswith("//"):
        package, _, name = test_target[2:].partition(":")
        return f"bazel-testlogs/{package}/{name}/test.outputs"
    if output_path:
        return os.path.dirname(os.path.abspath(output_path))
    return None


def main() -> None:
    """Main entry point for the orchestrator CLI."""
    parser = argument_parser()
    args = parser.parse_args()

    # When run as a Bazel test, write every report into the test's
    # undeclared-outputs directory automatically. This is the native test
    # "artifact" mechanism: unlike a build action a test cannot declare output
    # files, but the runner exposes $TEST_UNDECLARED_OUTPUTS_DIR and Bazel zips
    # its contents into bazel-testlogs/.../test.outputs/outputs.zip. Detecting
    # it here keeps the Bazel test launcher trivial (no output-path plumbing).
    test_output_dir = os.environ.get("TEST_UNDECLARED_OUTPUTS_DIR") or os.environ.get(
        "TEST_TMPDIR"
    )
    if test_output_dir:
        os.makedirs(test_output_dir, exist_ok=True)
        defaults = {
            "output": "analysis.json",
            "html": "analysis.html",
            "rst": "analysis.rst",
            "guidelines_output": "guidelines",
            "debug_log": "debug.log",
        }
        for attr, filename in defaults.items():
            if getattr(args, attr) is None:
                setattr(args, attr, os.path.join(test_output_dir, filename))

    orchestrator = AnalysisOrchestrator(
        model_name=args.model,
        guidelines_path=args.guidelines,
        guideline_files=args.guidelines_file or None,
        cache_dir=args.cache,
        debug_log=args.debug_log if args.verbose else None,
        batch_size=args.batch_size,
        custom_ai_model=args.custom_ai_model,
        max_concurrent_requests=args.max_concurrent_requests,
        max_batch_chars=args.max_batch_chars,
        context_files=args.context_file or None,
        project_guideline_files=args.project_guidelines or None,
    )
    analysis_results = orchestrator.analyze_directory(
        args.input,
        args.deps,
        req_files=args.req_file or None,
        artefact_type=args.artefact_type,
        puml_files=args.puml_file or None,
    )

    # Format and output results
    orchestrator.format_and_output(
        analysis_results,
        output_file=args.output,
        html_file=args.html,
        guidelines_output_dir=args.guidelines_output,
        rst_file=args.rst,
    )

    # Tell the user where to find the reports. Inside a Bazel test the reports
    # live under bazel-testlogs/<package>/<name>/test.outputs/ — a stable,
    # workspace-root-relative path that is far more useful than the sandbox
    # temp dir or the test launcher script. Derive it from $TEST_TARGET
    # (//package:name), which Bazel sets for every test.
    report_location = _report_location(args.output)
    if report_location:
        print(f"AI analysis reports: {report_location}")

    # Enforce the score threshold when requested (Bazel test rules). The
    # analysis itself runs as the test action, so a failing score fails the
    # test directly.
    if args.score_threshold is not None:
        scores = [a.score for a in analysis_results.analyses]
        average = sum(scores) / len(scores) if scores else 0.0
        if average < args.score_threshold:
            print(
                f"ERROR: Average score {average:.2f} is below threshold "
                f"{args.score_threshold:.2f}",
                file=sys.stderr,
            )
            sys.exit(1)
        print(
            f"AI analysis complete. Average score: {average:.2f} "
            f"(threshold: {args.score_threshold:.2f})"
        )


if __name__ == "__main__":
    main()
