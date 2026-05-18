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
"""Conversion between LangChain message types and Copilot SDK prompt format."""

from __future__ import annotations

import json
from typing import Optional

from langchain_core.messages import (
    AIMessage,
    BaseMessage,
    HumanMessage,
    SystemMessage,
    ToolMessage,
)


def messages_to_prompt(messages: list[BaseMessage]) -> str:
    """Convert a list of LangChain messages into a single prompt string.

    The Copilot SDK accepts a plain text prompt rather than a structured
    message array.  We serialise the conversation into a tagged format so
    the model can distinguish roles.
    """
    parts: list[str] = []
    for msg in messages:
        content = (
            msg.content if isinstance(msg.content, str) else json.dumps(msg.content)
        )

        if isinstance(msg, SystemMessage):
            parts.append(f"[system]\n{content}")
        elif isinstance(msg, HumanMessage):
            parts.append(f"[user]\n{content}")
        elif isinstance(msg, AIMessage):
            text_parts = [f"[assistant]\n{content}"] if content else ["[assistant]"]
            if msg.tool_calls:
                for tc in msg.tool_calls:
                    text_parts.append(
                        f"[tool_call id={tc['id']} name={tc['name']}]\n"
                        f"{json.dumps(tc['args'])}"
                    )
            parts.append("\n".join(text_parts))
        elif isinstance(msg, ToolMessage):
            parts.append(f"[tool_result id={msg.tool_call_id}]\n{content}")
        else:
            parts.append(f"[{msg.type}]\n{content}")

    return "\n\n".join(parts)


def extract_system_message(messages: list[BaseMessage]) -> Optional[str]:
    """Return the content of the first message if it is a SystemMessage."""
    if messages and isinstance(messages[0], SystemMessage):
        content = messages[0].content
        return content if isinstance(content, str) else json.dumps(content)
    return None
