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
"""HTML renderer for analysis reports (Jinja2 template + autoescaping)."""

import os

from jinja2 import Environment, FileSystemLoader, select_autoescape
from markupsafe import Markup

from ai_checker.reports.base import ReportRenderer
from ai_checker.reports.models import AnalysisReport
from ai_checker.reports.text_utils import (
    extract_severity,
    markdown_to_html,
    normalize_filename,
    text_to_html,
)

# Score buckets (out of 10) used for the colour-coded score badge.
_SCORE_HIGH_THRESHOLD = 8.0
_SCORE_MEDIUM_THRESHOLD = 5.0

_TEMPLATE_NAME = "report.html.j2"
_TEMPLATE_DIR = os.path.dirname(os.path.abspath(__file__))


def _score_class(score: float) -> str:
    """Map a 0-10 score to a CSS class: ``high`` / ``medium`` / ``low``."""
    if score >= _SCORE_HIGH_THRESHOLD:
        return "high"
    if score >= _SCORE_MEDIUM_THRESHOLD:
        return "medium"
    return "low"


def _build_environment() -> Environment:
    """Create the Jinja2 environment with autoescaping and report filters.

    ``markdown``/``text_br`` produce HTML deliberately, so they return
    ``Markup`` to opt out of the autoescaper — but they escape their input
    first (see ``text_utils``), so untrusted model text cannot inject HTML.
    Every other interpolation is autoescaped by default.
    """
    env = Environment(
        loader=FileSystemLoader(_TEMPLATE_DIR),
        autoescape=select_autoescape(["html", "j2"]),
        trim_blocks=True,
        lstrip_blocks=True,
    )
    env.filters["markdown"] = lambda text: Markup(markdown_to_html(text))
    env.filters["text_br"] = lambda text: Markup(text_to_html(text))
    env.filters["severity"] = extract_severity
    env.filters["score_class"] = _score_class
    return env


class HtmlRenderer(ReportRenderer):
    """Render an analysis report as a styled, self-contained HTML page."""

    extension = ".html"

    def __init__(self, guidelines_output_dir: str | None = None):
        # When set, guideline subpages are written here instead of a "guidelines"
        # directory beside the report.
        super().__init__(guidelines_output_dir=guidelines_output_dir)
        self._template = _build_environment().get_template(_TEMPLATE_NAME)

    def render(self, report: AnalysisReport) -> str:
        analyses = report.analyses
        total = len(analyses)
        avg_score = sum(a.score for a in analyses) / total if total else 0

        return self._template.render(
            artefact_type_title=report.metadata.artefact_type.capitalize(),
            metadata=report.metadata,
            total=total,
            avg_score=avg_score,
            guideline_links=self._guideline_links(report),
            analyses=analyses,
        )

    def _guidelines_dir(self, out_path: str) -> str:
        if self._guidelines_output_dir:
            return self._guidelines_output_dir
        return os.path.join(os.path.dirname(out_path), "guidelines")

    def _guideline_links(self, report: AnalysisReport) -> list[dict[str, str]]:
        """Return ``[{name, href}]`` for each guideline page (autoescaped later)."""
        if not report.guidelines:
            return []

        if self._out_path:
            report_dir = os.path.dirname(self._out_path)
            output_dir = self._guidelines_dir(self._out_path)
            try:
                relative_base = os.path.relpath(output_dir, report_dir)
            except ValueError:
                relative_base = output_dir
        else:
            relative_base = "guidelines"

        links = []
        for name in sorted(report.guidelines):
            slug = normalize_filename(name)
            links.append({"name": name, "href": f"{relative_base}/guideline_{slug}.md"})
        return links

    def write_extras(self, report: AnalysisReport, out_path: str) -> None:
        """Write one ``guideline_<slug>.md`` page per guideline."""
        if not report.guidelines:
            return
        output_dir = self._guidelines_dir(out_path)
        os.makedirs(output_dir, exist_ok=True)
        for name, content in report.guidelines.items():
            slug = normalize_filename(name)
            page_path = os.path.join(output_dir, f"guideline_{slug}.md")
            with open(page_path, "w", encoding="utf-8") as f:
                f.write(f"# {name}\n\n")
                f.write(content)
