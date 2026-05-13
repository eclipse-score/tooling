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
"""Conversion between LangChain tool specs and Copilot SDK Tool objects."""

from __future__ import annotations

import json
from collections.abc import Sequence
from typing import Any, Callable

from copilot.tools import Tool as CopilotTool, ToolInvocation, ToolResult
from langchain_core.tools import BaseTool
from langchain_core.utils.function_calling import convert_to_openai_tool


def convert_tools_to_openai_format(
    tools: Sequence[dict[str, Any] | type | Callable | BaseTool],
) -> list[dict[str, Any]]:
    """Convert LangChain tool specs to OpenAI-format tool definitions."""
    result = []
    for tool in tools:
        if isinstance(tool, dict):
            result.append(tool)
        else:
            result.append(convert_to_openai_tool(tool))
    return result


def build_copilot_tools(
    openai_tools: list[dict[str, Any]],
) -> list[CopilotTool]:
    """Convert OpenAI-format tool dicts into Copilot SDK Tool objects.

    The handler is a no-op because we never let the Copilot agent
    autonomously execute tools — we only need the definitions so the
    model can emit tool_calls in its response.
    """
    copilot_tools = []
    for t in openai_tools:
        fn = t.get("function", t)
        name = fn["name"]
        description = fn.get("description", "")
        parameters = fn.get("parameters")

        def _make_noop_handler(tool_name: str):
            async def _noop_handler(invocation: ToolInvocation) -> ToolResult:
                return ToolResult(
                    text_result_for_llm="Tool execution is managed by LangChain.",
                    result_type="success",
                )

            return _noop_handler

        copilot_tools.append(
            CopilotTool(
                name=name,
                description=description,
                handler=_make_noop_handler(name),
                parameters=parameters,
            )
        )
    return copilot_tools


def deep_decode_json_strings(obj: Any) -> Any:
    """Recursively decode values that are JSON-encoded strings.

    Some LLMs (e.g. Claude via the Copilot SDK) double-encode nested
    lists or objects as JSON strings inside the outer tool-call arguments
    dict.  This function walks the structure and replaces any string value
    that successfully parses as a JSON array or object with the decoded
    Python value, leaving plain strings untouched.
    """
    if isinstance(obj, dict):
        return {k: deep_decode_json_strings(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [deep_decode_json_strings(v) for v in obj]
    if isinstance(obj, str):
        stripped = obj.strip()
        if stripped and stripped[0] in ("{", "["):
            try:
                decoded = json.loads(stripped)
                if isinstance(decoded, (dict, list)):
                    return deep_decode_json_strings(decoded)
            except (json.JSONDecodeError, ValueError):
                pass
    return obj
