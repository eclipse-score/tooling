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
Data models for requirement analysis results.

This module provides Pydantic models for structured analysis outputs.
"""

from pydantic import BaseModel, Field


class RequirementAnalysis(BaseModel):
    """Structured output for individual requirement analysis."""

    requirement_id: str = Field(description="Unique identifier for the requirement")
    description: str = Field(
        description=("Brief description of the requirement (first line is sufficient)")
    )
    findings: list[str] = Field(description="List of findings from the analysis")
    suggestions: list[str] = Field(description="List of suggestions from the analysis")
    score: float = Field(
        description="Numerical score from 0 to 10 representing analysis quality",
        ge=0.0,
        le=10,
    )


class AnalysisResults(BaseModel):
    """Structured output for multiple requirement analyses."""

    analyses: list[RequirementAnalysis] = Field(
        description="List of requirement analyses"
    )
