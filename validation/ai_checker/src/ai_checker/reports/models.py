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
Self-contained report model.

``AnalysisReport`` is the canonical, fully self-contained representation of a
finished analysis: report-wide metadata, the guideline texts used, and the
per-requirement analyses. Everything a renderer needs lives here, so the same
object can be rendered to JSON / HTML / reST without any external state.

Each per-requirement record is the existing ``RequirementAnalysis``; its
``description`` field holds the full original artefact description (the formatter
swaps the AI's brief description for the original when one is available).
"""

from pydantic import BaseModel, Field

from ai_checker.analysis_models import RequirementAnalysis


class ReportMetadata(BaseModel):
    """Report-wide metadata."""

    model_name: str = Field(description="AI model used for the analysis")
    timestamp: str = Field(description="ISO timestamp when the report was produced")
    git_hash: str = Field(description="Git commit hash of the analyzed sources")
    artefact_type: str = Field(
        default="requirements",
        description="Artefact type analyzed ('requirements' or 'architecture')",
    )


class AnalysisReport(BaseModel):
    """Everything needed to render an analysis report."""

    metadata: ReportMetadata
    guidelines: dict[str, str] = Field(
        default_factory=dict,
        description="Guideline name -> markdown content (for subpages and links)",
    )
    analyses: list[RequirementAnalysis] = Field(
        default_factory=list,
        description="Per-requirement analyses (description = full original text)",
    )
