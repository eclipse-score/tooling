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
Facade that builds an :class:`AnalysisReport` and renders it.

It assembles **one** report object in memory from the analysis results plus
metadata and guideline texts, then renders the requested format directly from
that object. JSON is just one of the renderers — HTML / reST are produced from
the same in-memory report, never by re-reading the JSON.
"""

import os
from typing import Any, Dict, Optional

from ai_checker.analysis_models import AnalysisResults
from ai_checker.guidelines_reader import GuidelinesReader
from ai_checker.reports.base import ReportRenderer
from ai_checker.reports.html_renderer import HtmlRenderer
from ai_checker.reports.json_renderer import JsonRenderer
from ai_checker.reports.metadata import get_git_hash, get_timestamp
from ai_checker.reports.models import AnalysisReport, ReportMetadata
from ai_checker.reports.rst_renderer import RstRenderer

import logging

# Extension -> renderer class. Anything else falls back to JSON with a warning.
_RENDERERS: dict[str, type[ReportRenderer]] = {
    ".html": HtmlRenderer,
    ".rst": RstRenderer,
    ".json": JsonRenderer,
}

logger = logging.getLogger(__name__)


class ResultFormatter:
    """Builds an :class:`AnalysisReport` and writes it in the requested format."""

    def __init__(
        self,
        analysis_results: AnalysisResults,
        model_name: Optional[str] = None,
        guidelines_reader: Optional[GuidelinesReader] = None,
        guidelines_output_dir: Optional[str] = None,
        original_requirements: Optional[Dict[str, Dict[str, Any]]] = None,
        artefact_type: str = "requirements",
    ):
        """
        Args:
            analysis_results: AnalysisResults object containing analyses
            model_name: Name of the AI model used for analysis
            guidelines_reader: GuidelinesReader holding the guideline texts
            guidelines_output_dir: Optional directory for guideline subpages
            original_requirements: Original artefact data {id: {metadata}}; the
                full ``description`` from here replaces the AI's brief one.
            artefact_type: 'requirements' or 'architecture' (report metadata)
        """
        self.guidelines_output_dir = guidelines_output_dir
        self.report = self._build_report(
            analysis_results,
            model_name or "Unknown",
            guidelines_reader,
            original_requirements or {},
            artefact_type,
        )

    @staticmethod
    def _build_report(
        analysis_results: AnalysisResults,
        model_name: str,
        guidelines_reader: Optional[GuidelinesReader],
        original_requirements: Dict[str, Dict[str, Any]],
        artefact_type: str,
    ) -> AnalysisReport:
        # Swap each analysis's brief description for the full original when known.
        analyses = []
        for analysis in analysis_results.analyses:
            original = original_requirements.get(analysis.requirement_id, {})
            full_desc = original.get("description")
            if full_desc:
                analyses.append(analysis.model_copy(update={"description": full_desc}))
            else:
                analyses.append(analysis)

        guidelines = dict(guidelines_reader.guidelines) if guidelines_reader else {}

        return AnalysisReport(
            metadata=ReportMetadata(
                model_name=model_name,
                timestamp=get_timestamp(),
                git_hash=get_git_hash(),
                artefact_type=artefact_type,
            ),
            guidelines=guidelines,
            analyses=analyses,
        )

    def output(self, file_path: Optional[str] = None) -> None:
        """Write the report. ``None`` prints the JSON envelope to stdout.

        The output format is chosen by ``file_path``'s extension (``.html``,
        ``.rst``, else JSON).
        """
        if file_path is None:
            print(JsonRenderer().render(self.report))
            return

        parent = os.path.dirname(file_path)
        if parent:
            os.makedirs(parent, exist_ok=True)

        extension = os.path.splitext(file_path)[1].lower()
        renderer = self._make_renderer(extension)

        # Every renderer accepts an output path so it can compute relative links
        # to companion files; renderers that emit a self-contained document
        # simply ignore it.
        renderer._out_path = file_path

        with open(file_path, "w", encoding="utf-8") as f:
            f.write(renderer.render(self.report))
        renderer.write_extras(self.report, file_path)

        # Under a Bazel test the absolute sandbox path is noise (and there is one
        # such line per renderer); the orchestrator prints a single
        # workspace-relative location instead. Only report the path for direct
        # CLI use, where the absolute path is genuinely useful.
        if not os.environ.get("TEST_UNDECLARED_OUTPUTS_DIR"):
            print(f"Analysis results written to {file_path}")

    def _make_renderer(self, extension: str) -> ReportRenderer:
        renderer_cls = _RENDERERS.get(extension)
        if renderer_cls is None:
            logger.warning(
                "Unknown report extension %r — writing JSON content to this "
                "file regardless of its extension.",
                extension,
            )
            renderer_cls = JsonRenderer
        # All renderers share the base constructor signature; those that don't
        # emit companion guideline pages simply ignore the directory.
        return renderer_cls(guidelines_output_dir=self.guidelines_output_dir)
