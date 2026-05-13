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
LangChain BaseChatModel wrapper for the GitHub Copilot SDK.

Provides a fully LangChain-compatible chat model that supports:
- Standard message types (system, human, AI, tool)
- Tool calling via bind_tools()
- Structured output via with_structured_output()
- Async generation (native)
- Sync generation (via asyncio bridge)
"""

from __future__ import annotations

import asyncio
import concurrent.futures
import json
import logging
from collections.abc import Sequence
from typing import Any, Callable, Optional

from copilot.generated.session_events import ExternalToolRequestedData, SessionEventType
from copilot.session import PermissionHandler, SessionConfig

from langchain_core.callbacks import (
    AsyncCallbackManagerForLLMRun,
    CallbackManagerForLLMRun,
)
from langchain_core.language_models.chat_models import BaseChatModel
from langchain_core.messages import AIMessage, BaseMessage, SystemMessage
from langchain_core.outputs import ChatGeneration, ChatResult
from langchain_core.tools import BaseTool
from pydantic import Field, PrivateAttr

from ._client_manager import CopilotClientManager
from ._errors import CopilotSetupError
from ._message_converter import extract_system_message, messages_to_prompt
from ._preflight import describe_auth_sources
from ._tool_converter import (
    build_copilot_tools,
    convert_tools_to_openai_format,
    deep_decode_json_strings,
)

logger = logging.getLogger(__name__)


class ChatCopilot(BaseChatModel):
    """LangChain chat model backed by the GitHub Copilot SDK.

    Example:
        >>> from copilot_langchain import ChatCopilot
        >>>
        >>> llm = ChatCopilot(model="gpt-4.1")
        >>> response = await llm.ainvoke("Hello, how are you?")
        >>> print(response.content)

    With tools:
        >>> from langchain_core.tools import tool
        >>>
        >>> @tool
        >>> def add(a: int, b: int) -> int:
        ...     \"\"\"Add two numbers.\"\"\"
        ...     return a + b
        >>>
        >>> llm_with_tools = ChatCopilot(model="gpt-4.1").bind_tools([add])
        >>> response = await llm_with_tools.ainvoke("What is 2 + 3?")

    With structured output:
        >>> from pydantic import BaseModel
        >>>
        >>> class Answer(BaseModel):
        ...     value: int
        ...     explanation: str
        >>>
        >>> chain = ChatCopilot(model="gpt-4.1").with_structured_output(Answer)
        >>> result = await chain.ainvoke("What is 2 + 2?")
        >>> print(result.value)
    """

    model: str = "gpt-4.1"
    """Model identifier to use (e.g. 'gpt-4.1', 'claude-sonnet-4')."""

    timeout: float = 120.0
    """Timeout in seconds for waiting on a response."""

    copilot_client_options: dict[str, Any] = Field(default_factory=dict)
    """Options passed to CopilotClient() constructor."""

    # Private attributes (not serialised by Pydantic)
    _manager: CopilotClientManager = PrivateAttr(default=None)
    _bound_tools: list[dict[str, Any]] = PrivateAttr(default_factory=list)
    _tool_choice: Optional[str] = PrivateAttr(default=None)
    _ls_structured_output_format: Optional[dict[str, Any]] = PrivateAttr(default=None)

    def model_post_init(self, __context: Any) -> None:
        self._manager = CopilotClientManager(self.copilot_client_options)

    # ------------------------------------------------------------------ #
    # LangChain required properties
    # ------------------------------------------------------------------ #

    @property
    def _llm_type(self) -> str:
        return "copilot-sdk"

    @property
    def _identifying_params(self) -> dict[str, Any]:
        return {"model": self.model}

    # ------------------------------------------------------------------ #
    # Lifecycle
    # ------------------------------------------------------------------ #

    async def aclose(self) -> None:
        """Shut down the underlying Copilot CLI process."""
        await self._manager.close()

    # ------------------------------------------------------------------ #
    # Structured output (JSON-based, bypasses tool calling)
    # ------------------------------------------------------------------ #

    def with_structured_output(
        self,
        schema: Any,
        *,
        include_raw: bool = False,
        **kwargs: Any,
    ) -> Any:
        """Return a chain that produces structured output via JSON text parsing.

        The Copilot CLI's model ignores tool-calling instructions and produces
        natural language responses even when tools are registered. This override
        injects a JSON schema requirement directly into the system prompt and
        parses the model's text response, which is far more reliable.
        """
        from pydantic import BaseModel as PydanticBaseModel

        from langchain_core.runnables import RunnableLambda

        is_pydantic = isinstance(schema, type) and issubclass(schema, PydanticBaseModel)
        schema_json = schema.model_json_schema() if is_pydantic else schema
        schema_str = json.dumps(schema_json, indent=2)

        json_instruction = (
            "\n\n# CRITICAL OUTPUT FORMAT REQUIREMENT\n"
            "You MUST respond with ONLY a valid JSON object. No prose, no markdown, "
            "no explanations, no code fences.\n"
            "Your ENTIRE response must be a single valid JSON object matching this schema:\n"
            f"{schema_str}\n"
            "Start your response immediately with `{` and end with `}`."
        )

        def _inject(messages: list[BaseMessage]) -> list[BaseMessage]:
            out: list[BaseMessage] = []
            injected = False
            for msg in messages:
                if isinstance(msg, SystemMessage) and not injected:
                    out.append(SystemMessage(content=msg.content + json_instruction))
                    injected = True
                else:
                    out.append(msg)
            if not injected:
                out.insert(0, SystemMessage(content=json_instruction.lstrip()))
            return out

        def _parse(ai_message: AIMessage) -> Any:
            content = (ai_message.content or "").strip()
            # Strip markdown code fences if present
            if content.startswith("```"):
                lines = content.split("\n")
                content = "\n".join(lines[1:-1]).strip()
            # Extract outermost JSON object
            start = content.find("{")
            end = content.rfind("}") + 1
            if start == -1 or end == 0:
                raise ValueError(
                    f"No JSON object found in model response.\n"
                    f"--- LLM output ---\n{content}\n--- end ---"
                )
            json_text = content[start:end]
            try:
                parsed = json.loads(json_text)
            except json.JSONDecodeError as exc:
                raise ValueError(
                    f"Model returned invalid JSON: {exc}\n"
                    f"--- LLM output ---\n{content}\n--- end ---"
                ) from exc
            if not is_pydantic:
                return parsed
            try:
                return schema.model_validate(parsed)
            except Exception as exc:
                raise ValueError(
                    f"Model output did not match the expected schema: {exc}\n"
                    f"--- LLM output ---\n{content}\n--- end ---"
                ) from exc

        chain = RunnableLambda(_inject) | self | RunnableLambda(_parse)
        return chain

    # ------------------------------------------------------------------ #
    # Tool binding
    # ------------------------------------------------------------------ #

    def bind_tools(
        self,
        tools: Sequence[dict[str, Any] | type | Callable | BaseTool],
        *,
        tool_choice: str | None = None,
        **kwargs: Any,
    ) -> ChatCopilot:
        """Return a new ChatCopilot with tools bound.

        Args:
            tools: Tools to make available to the model.
            tool_choice: When set to "any", forces the model to use a tool.
                Used internally by with_structured_output().

        Returns:
            A new ChatCopilot instance with the tools bound.
        """
        openai_tools = convert_tools_to_openai_format(tools)
        new = self.model_copy()
        new._bound_tools = openai_tools
        new._tool_choice = tool_choice
        new._ls_structured_output_format = kwargs.get("ls_structured_output_format")
        # Share the same client manager so the subprocess is not restarted
        new._manager = self._manager
        return new

    # ------------------------------------------------------------------ #
    # Core generation (async — the native path)
    # ------------------------------------------------------------------ #

    async def _agenerate(
        self,
        messages: list[BaseMessage],
        stop: list[str] | None = None,
        run_manager: AsyncCallbackManagerForLLMRun | None = None,
        **kwargs: Any,
    ) -> ChatResult:
        try:
            client = await self._manager.ensure_client()
        except CopilotSetupError:
            raise
        except Exception as exc:
            raise CopilotSetupError(
                f"Unexpected error initialising Copilot SDK: {type(exc).__name__}: {exc}\n\n"
                + describe_auth_sources()
            ) from exc

        # Build session config
        session_config: SessionConfig = {
            "model": kwargs.get("model", self.model),
            "available_tools": [],  # Disable built-in tools
        }

        # Merge any extra tools from kwargs with bound tools
        extra_tools = kwargs.get("tools", [])
        all_openai_tools = self._bound_tools + (
            convert_tools_to_openai_format(extra_tools) if extra_tools else []
        )
        if all_openai_tools:
            session_config["tools"] = build_copilot_tools(all_openai_tools)

        # System message
        system_content = extract_system_message(messages)
        if system_content:
            base_system = system_content
            prompt_messages = [m for m in messages if not isinstance(m, SystemMessage)]
        else:
            base_system = "You are a helpful assistant."
            prompt_messages = messages

        session_config["system_message"] = {"mode": "replace", "content": base_system}
        session_config["infinite_sessions"] = {"enabled": False}

        session = await client.create_session(
            on_permission_request=PermissionHandler.approve_all,
            **session_config,
        )
        try:
            prompt = messages_to_prompt(prompt_messages)

            tool_requests: list[Any] = []

            def _event_handler(event: Any) -> None:
                # New SDK (protocol v3): custom tool calls come as ExternalToolRequestedData
                # broadcast events rather than AssistantMessageData.tool_requests.
                if isinstance(event.data, ExternalToolRequestedData):
                    tool_requests.append(event.data)
                    return
                # Legacy fallback: tool calls in AssistantMessageData.tool_requests
                if event.type == SessionEventType.ASSISTANT_MESSAGE:
                    if event.data.tool_requests:
                        tool_requests.extend(event.data.tool_requests)

            unsubscribe = session.on(_event_handler)
            try:
                response = await session.send_and_wait(
                    prompt,
                    timeout=self.timeout,
                )
            finally:
                unsubscribe()

            content = ""
            if response and response.data and response.data.content:
                content = response.data.content

            if response and response.data and response.data.tool_requests:
                for tr in response.data.tool_requests:
                    if tr not in tool_requests:
                        tool_requests.append(tr)

            tool_calls = []
            seen_ids: set[str] = set()
            for tr in tool_requests:
                if isinstance(tr, ExternalToolRequestedData):
                    name, call_id, args = tr.tool_name, tr.tool_call_id, tr.arguments
                else:
                    name, call_id, args = tr.name, tr.tool_call_id, tr.arguments
                if call_id in seen_ids:
                    continue
                seen_ids.add(call_id)
                if isinstance(args, str):
                    try:
                        args = json.loads(args)
                    except (json.JSONDecodeError, TypeError):
                        args = {"raw": args}
                elif args is None:
                    args = {}
                if isinstance(args, dict):
                    args = deep_decode_json_strings(args)
                tool_calls.append(
                    {
                        "name": name,
                        "args": args if isinstance(args, dict) else {"raw": args},
                        "id": call_id,
                    }
                )

            ai_message = AIMessage(
                content=content,
                tool_calls=tool_calls if tool_calls else [],
                response_metadata={"model": self.model},
            )
            return ChatResult(generations=[ChatGeneration(message=ai_message)])
        finally:
            await session.destroy()

    # ------------------------------------------------------------------ #
    # Sync generation (bridges to async)
    # ------------------------------------------------------------------ #

    def _generate(
        self,
        messages: list[BaseMessage],
        stop: list[str] | None = None,
        run_manager: CallbackManagerForLLMRun | None = None,
        **kwargs: Any,
    ) -> ChatResult:
        """Synchronous generation — delegates to the async implementation."""
        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            loop = None

        if loop and loop.is_running():
            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                future = pool.submit(
                    asyncio.run,
                    self._agenerate(messages, stop, None, **kwargs),
                )
                return future.result()
        else:
            return asyncio.run(self._agenerate(messages, stop, None, **kwargs))
