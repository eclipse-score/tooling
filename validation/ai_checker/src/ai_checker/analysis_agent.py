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
Agent interface for AI artefact analysis.

This module defines the thin contract the core analysis loop relies on. An
implementation receives a system prompt (guidelines + optional background
context) and a formatted artefacts string, and returns structured
``AnalysisResults``.

The interface is intentionally minimal: it decouples the core (batching,
caching, concurrency) from any particular AI SDK. The default implementation
talks to the GitHub Copilot SDK directly (``CopilotAgent``); a LangChain
adapter (``LangChainAgent``) is provided for SDKs exposed as a LangChain
``BaseChatModel``.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass

from ai_checker.analysis_models import AnalysisResults


@dataclass(frozen=True)
class Usage:
    """Accumulated AI usage for a run.

    A small, SDK-agnostic value object so the core loop never has to probe
    backend-specific response fields. ``ai_credits`` is the GitHub Copilot
    billing unit (AIU); ``cost_usd`` is a best-effort dollar figure when the
    backend reports one. Any field a backend cannot report stays ``0``.
    """

    tokens: int = 0
    cost_usd: float = 0.0
    ai_credits: float = 0.0

    def __add__(self, other: "Usage") -> "Usage":
        return Usage(
            tokens=self.tokens + other.tokens,
            cost_usd=self.cost_usd + other.cost_usd,
            ai_credits=self.ai_credits + other.ai_credits,
        )

    @property
    def is_empty(self) -> bool:
        """True when no usage at all was reported."""
        return not (self.tokens or self.cost_usd or self.ai_credits)


class AnalysisAgent(ABC):
    """Abstract AI backend that produces structured analysis results.

    Implementations should be safe to call concurrently from multiple
    coroutines; the core analysis loop fans out batches with
    ``asyncio.gather``.

    Usage accounting: implementations that can report AI usage call
    :meth:`_record_usage` after each request; the core loop reads the running
    total via :meth:`get_usage`. Implementations that cannot report usage
    simply never call ``_record_usage`` and ``get_usage`` stays empty.
    """

    def __init__(self) -> None:
        # Instance-level usage accumulator. Defined here (not as a class
        # attribute) so every agent starts at zero independently and no
        # subclass can accidentally share or mutate a class-level value.
        self._usage = Usage()

    def get_usage(self) -> Usage:
        """Return the usage accumulated across all ``analyze`` calls so far."""
        return self._usage

    def _record_usage(self, usage: Usage) -> None:
        """Add ``usage`` to the running total.

        Safe to call from the concurrent ``analyze`` coroutines: the add is a
        single synchronous statement with no ``await`` in between, so under
        asyncio's cooperative scheduling it cannot interleave.
        """
        self._usage = self._usage + usage

    @abstractmethod
    async def analyze(self, system_prompt: str, artefacts_text: str) -> AnalysisResults:
        """Analyze a batch of artefacts against the system prompt.

        Args:
            system_prompt: Combined system instructions — guidelines plus any
                background context — used as the system message.
            artefacts_text: Formatted artefacts to analyze (the user message).

        Returns:
            Structured ``AnalysisResults`` for the artefacts in this batch.
        """
        raise NotImplementedError

    async def aclose(self) -> None:
        """Release any resources held by the agent (e.g. CLI subprocesses).

        Default is a no-op. Implementations that own external resources must
        override this; the orchestrator always calls it after analysis so
        cleanup is guaranteed regardless of the concrete backend.
        """
        return None
