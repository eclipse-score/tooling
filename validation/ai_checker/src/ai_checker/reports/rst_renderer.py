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
"""Standalone reStructuredText renderer for analysis reports."""

import os

from ai_checker.reports.base import ReportRenderer
from ai_checker.reports.models import AnalysisReport
from ai_checker.reports.text_utils import normalize_filename, strip_markup


def _heading(text: str, char: str) -> str:
    """reST heading: text on one line, underline of ``char`` of equal length."""
    return f"{text}\n{char * len(text)}\n"


def _bullets(items: list[str]) -> str:
    """Render a reST bullet list."""
    lines = []
    for item in items:
        text = strip_markup(item).replace("\n", " ")
        lines.append(f"- {text}")
    return "\n".join(lines) + "\n"


class RstRenderer(ReportRenderer):
    """Render an analysis report as standalone reStructuredText (no docutils)."""

    extension = ".rst"

    def __init__(self, guidelines_output_dir: str | None = None):
        super().__init__(guidelines_output_dir=guidelines_output_dir)

    def render(self, report: AnalysisReport) -> str:
        analyses = report.analyses
        total = len(analyses)
        avg_score = sum(a.score for a in analyses) / total if total else 0
        meta = report.metadata

        parts: list[str] = []
        title = f"{meta.artefact_type.capitalize()} Analysis Report"
        parts.append(_heading(title, "="))
        parts.append("")
        parts.append(f":Total artefacts: {total}")
        parts.append(f":Average score: {avg_score:.1f}/10")
        parts.append(f":AI model: {meta.model_name}")
        parts.append(f":Hash: {meta.git_hash}")
        parts.append(f":Timestamp: {meta.timestamp}")
        parts.append("")

        # Guidelines
        parts.append(_heading("Guidelines used", "-"))
        if report.guidelines:
            base = self._relative_base()
            for name in sorted(report.guidelines):
                slug = normalize_filename(name)
                parts.append(f"- `{name} <{base}/guideline_{slug}.rst>`_")
        else:
            parts.append("- No guidelines specified")
        parts.append("")

        # Per-artefact sections
        for analysis in analyses:
            parts.append(
                _heading(f"{analysis.requirement_id} ({analysis.score:.1f}/10)", "-")
            )
            parts.append("")
            desc = strip_markup(analysis.description).replace("\n", " ")
            parts.append(f"**Description:** {desc}")
            parts.append("")
            if analysis.findings:
                parts.append("Findings:")
                parts.append("")
                parts.append(_bullets(analysis.findings))
            if analysis.suggestions:
                parts.append("Suggestions:")
                parts.append("")
                parts.append(_bullets(analysis.suggestions))

        return "\n".join(parts).rstrip() + "\n"

    def _guidelines_dir(self, out_path: str) -> str:
        if self._guidelines_output_dir:
            return self._guidelines_output_dir
        return os.path.join(os.path.dirname(out_path), "guidelines")

    def _relative_base(self) -> str:
        if self._out_path:
            report_dir = os.path.dirname(self._out_path)
            output_dir = self._guidelines_dir(self._out_path)
            try:
                return os.path.relpath(output_dir, report_dir)
            except ValueError:
                return output_dir
        return "guidelines"

    def write_extras(self, report: AnalysisReport, out_path: str) -> None:
        """Write one ``guideline_<slug>.rst`` page per guideline."""
        if not report.guidelines:
            return
        output_dir = self._guidelines_dir(out_path)
        os.makedirs(output_dir, exist_ok=True)
        for name, content in report.guidelines.items():
            slug = normalize_filename(name)
            page_path = os.path.join(output_dir, f"guideline_{slug}.rst")
            with open(page_path, "w", encoding="utf-8") as f:
                f.write(_heading(name, "="))
                f.write("\n::\n\n")
                # Embed source guideline verbatim as a literal block (it is
                # markdown, not reST, so quote it to avoid parse errors).
                for line in content.splitlines():
                    f.write(f"   {line}\n")
