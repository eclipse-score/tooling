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
Direct GitHub Copilot SDK implementation of :class:`AnalysisAgent`.

This is the default AI backend. Unlike the optional LangChain adapter
(``LangChainAgent``), it talks to the Copilot SDK directly: it owns a single CLI session per request,
embeds the required JSON schema in the system prompt, and parses the model's
text reply into ``AnalysisResults``. No LangChain dependency is involved.

The JSON-in-prompt approach (rather than tool calling) is used because the
Copilot CLI's model responds in plain text and ignores tool-calling
instructions — see ``validation/ai_checker/DEVELOPMENT.md``.
"""

from __future__ import annotations

import json
import logging
from typing import Any

from copilot.session import PermissionHandler
from copilot.session_events import AssistantUsageData

from ai_checker.analysis_agent import AnalysisAgent, Usage
from ai_checker.analysis_models import AnalysisResults
from ai_checker.constants import DEFAULT_MODEL

from ._client_manager import CopilotClientManager
from ._errors import CopilotSetupError
from ._preflight import describe_auth_sources

logger = logging.getLogger(__name__)


def _build_json_instruction(schema: type) -> str:
    """Build the system-prompt suffix forcing a single JSON object reply."""
    schema_str = json.dumps(schema.model_json_schema(), indent=2)
    return (
        "\n\n# CRITICAL OUTPUT FORMAT REQUIREMENT\n"
        "You MUST respond with ONLY a valid JSON object. No prose, no markdown, "
        "no explanations, no code fences.\n"
        "Your ENTIRE response must be a single valid JSON object matching this schema:\n"
        f"{schema_str}\n"
        "Start your response immediately with `{` and end with `}`."
    )


def _extract_json_object(text: str) -> str:
    """Return the first balanced top-level ``{...}`` object in ``text``.

    Scans for the first ``{`` and tracks brace depth, ignoring braces inside
    double-quoted strings (with escape handling). This is more robust than a
    naive ``find('{')`` / ``rfind('}')`` when the model emits trailing prose or
    multiple objects after the intended one.

    Raises ``ValueError`` if no balanced object is found.
    """
    start = text.find("{")
    if start == -1:
        raise ValueError("No JSON object found in model response.")

    depth = 0
    in_string = False
    escaped = False
    for i in range(start, len(text)):
        ch = text[i]
        if in_string:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_string = False
            continue
        if ch == '"':
            in_string = True
        elif ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return text[start : i + 1]

    raise ValueError("Unterminated JSON object in model response.")


def _parse_results(content: str) -> AnalysisResults:
    """Parse the model's text reply into ``AnalysisResults``.

    Extracts the first balanced JSON object (tolerating surrounding prose or
    markdown code fences) and validates it against the schema. Raises
    ``ValueError`` with the full raw output on any failure.
    """
    content = (content or "").strip()
    try:
        json_text = _extract_json_object(content)
    except ValueError as exc:
        raise ValueError(f"{exc}\n--- LLM output ---\n{content}\n--- end ---") from exc
    try:
        parsed = json.loads(json_text)
    except json.JSONDecodeError as exc:
        raise ValueError(
            f"Model returned invalid JSON: {exc}\n"
            f"--- LLM output ---\n{content}\n--- end ---"
        ) from exc
    try:
        return AnalysisResults.model_validate(parsed)
    except Exception as exc:
        raise ValueError(
            f"Model output did not match the expected schema: {exc}\n"
            f"--- LLM output ---\n{content}\n--- end ---"
        ) from exc


# Conversion factor from the SDK's nano-AIU billing unit to whole AI credits
# (AIU). copilot_usage.total_nano_aiu is reported in 1e-9 AIU increments.
_NANO_AIU_PER_AIU = 1_000_000_000.0


def _usage_from_event(data: AssistantUsageData) -> Usage:
    """Convert one ``assistant.usage`` event into a :class:`Usage`.

    Token counts (``input_tokens`` / ``output_tokens``) and the experimental
    ``cost`` (USD) are stable public fields. GitHub Copilot AI credits (AIU)
    come from the ``copilot_usage`` billing block, which the SDK marks as an
    internal field (``_copilot_usage``) outside its public surface. It is read
    defensively via ``getattr`` so a future SDK rename or removal degrades to
    zero credits instead of raising — this function is the single isolation
    point for that internal dependency.
    """
    tokens = (data.input_tokens or 0) + (data.output_tokens or 0)
    cost_usd = data.cost or 0.0
    ai_credits = 0.0
    copilot_usage = getattr(data, "_copilot_usage", None)
    total_nano_aiu = getattr(copilot_usage, "total_nano_aiu", None)
    if total_nano_aiu:
        ai_credits = total_nano_aiu / _NANO_AIU_PER_AIU
    return Usage(tokens=tokens, cost_usd=cost_usd, ai_credits=ai_credits)


class CopilotAgent(AnalysisAgent):
    """Default :class:`AnalysisAgent` backed directly by the GitHub Copilot SDK."""

    def __init__(
        self,
        model: str = DEFAULT_MODEL,
        timeout: float = 120.0,
        copilot_client_options: dict[str, Any] | None = None,
    ) -> None:
        super().__init__()
        self.model = model
        self.timeout = timeout
        self._manager = CopilotClientManager(copilot_client_options or {})
        self._json_instruction = _build_json_instruction(AnalysisResults)

    async def aclose(self) -> None:
        """Shut down the underlying Copilot CLI process."""
        await self._manager.close()

    async def analyze(self, system_prompt: str, artefacts_text: str) -> AnalysisResults:
        try:
            client = await self._manager.ensure_client()
        except CopilotSetupError:
            raise
        except Exception as exc:
            raise CopilotSetupError(
                f"Unexpected error initialising Copilot SDK: "
                f"{type(exc).__name__}: {exc}\n\n" + describe_auth_sources()
            ) from exc

        system_content = system_prompt + self._json_instruction
        session_config: dict[str, Any] = {
            "model": self.model,
            "available_tools": [],  # Disable built-in tools
            "system_message": {"mode": "replace", "content": system_content},
            "infinite_sessions": {"enabled": False},
        }

        session = await client.create_session(
            on_permission_request=PermissionHandler.approve_all,
            **session_config,
        )

        # Accumulate usage from assistant.usage events emitted during this
        # request. send_and_wait only returns the final assistant message, so
        # usage (tokens / cost / AI credits) must be collected via the event
        # stream. The handler runs synchronously on the same event loop, so the
        # in-place add cannot interleave.
        request_usage = Usage()

        def _on_event(event: Any) -> None:
            nonlocal request_usage
            if isinstance(event.data, AssistantUsageData):
                request_usage = request_usage + _usage_from_event(event.data)

        unsubscribe = session.on(_on_event)
        try:
            response = await session.send_and_wait(
                artefacts_text,
                timeout=self.timeout,
            )

            content = ""
            if response and response.data and response.data.content:
                content = response.data.content

            if request_usage.is_empty:
                logger.debug("Usage not reported by Copilot SDK for this request.")
            else:
                self._record_usage(request_usage)

            return _parse_results(content)
        finally:
            unsubscribe()
            await session.disconnect()
