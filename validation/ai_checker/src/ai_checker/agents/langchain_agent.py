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
LangChain-backed implementation of :class:`AnalysisAgent`.

This adapter lets any LangChain ``BaseChatModel`` (e.g. ``ChatOpenAI``) serve as
the AI backend. It is **not** on the default path — the default is the direct
``CopilotAgent``. Use this only for SDKs exposed as a LangChain chat model,
supplied via a custom ``create_agent()`` hook.
"""

from __future__ import annotations

import logging

from langchain_core.language_models.chat_models import BaseChatModel
from langchain_core.messages import HumanMessage, SystemMessage

from ai_checker.analysis_agent import AnalysisAgent, Usage
from ai_checker.analysis_models import AnalysisResults

logger = logging.getLogger(__name__)


class LangChainAgent(AnalysisAgent):
    """Wrap a LangChain ``BaseChatModel`` as an :class:`AnalysisAgent`."""

    def __init__(self, chat_model: BaseChatModel) -> None:
        super().__init__()
        self._chat_model = chat_model
        self._structured = chat_model.with_structured_output(AnalysisResults)

    async def analyze(self, system_prompt: str, artefacts_text: str) -> AnalysisResults:
        response = await self._structured.ainvoke(
            [
                SystemMessage(content=system_prompt),
                HumanMessage(content=artefacts_text),
            ]
        )

        if not hasattr(response, "analyses") or not response.analyses:
            raise ValueError(
                "AI model returned empty or invalid response. "
                f"Expected 'analyses' field, got: {response}"
            )

        # Usage accounting is intentionally not attempted here. ``with_structured_output``
        # returns the parsed pydantic object, not the underlying ``AIMessage``, so the
        # per-call ``usage_metadata`` LangChain would otherwise expose is not reachable
        # from ``response``. Rather than read attributes that a standard ``BaseChatModel``
        # does not define (which always yielded zero), this adapter reports no usage and
        # ``get_usage()`` stays empty. Backends that need usage should use ``CopilotAgent``
        # or supply a custom agent that records it via ``_record_usage``.
        logger.debug(
            "LangChainAgent does not report usage; structured output omits "
            "the underlying message metadata."
        )

        return response
