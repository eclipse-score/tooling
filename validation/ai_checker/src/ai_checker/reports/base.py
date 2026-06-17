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
"""Renderer interface for analysis reports."""

from abc import ABC, abstractmethod

from ai_checker.reports.models import AnalysisReport


class ReportRenderer(ABC):
    """Render an :class:`AnalysisReport` to a single output format.

    Implementations are pure functions of the report — no external state — so
    the same report object can be rendered to any format.
    """

    #: File extension this renderer produces (including the leading dot).
    extension: str = ""

    def __init__(self, guidelines_output_dir: str | None = None) -> None:
        # Optional directory for companion guideline pages; ignored by
        # renderers that emit a self-contained document (e.g. JSON).
        self._guidelines_output_dir = guidelines_output_dir
        # Set by the caller before render() when relative links to companion
        # files must be computed from the final output location.
        self._out_path: str | None = None

    @abstractmethod
    def render(self, report: AnalysisReport) -> str:
        """Return the rendered document as a string."""
        raise NotImplementedError

    def write_extras(self, report: AnalysisReport, out_path: str) -> None:
        """Write any companion files next to ``out_path`` (e.g. guideline pages).

        Default: no extras.
        """
        return None
