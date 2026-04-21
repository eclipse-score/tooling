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
import json
import logging
import os
import stat
import uuid
from collections.abc import Sequence
from pathlib import Path
from typing import Any, Callable, Optional

from copilot import CopilotClient
from copilot.generated.session_events import SessionEvent, SessionEventType
from copilot.types import SessionConfig, Tool as CopilotTool, ToolInvocation, ToolResult

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Auth-related environment variables checked by the Copilot CLI (priority order)
# ---------------------------------------------------------------------------
_AUTH_ENV_VARS = [
    "COPILOT_GITHUB_TOKEN",  # Recommended for explicit Copilot usage
    "GH_TOKEN",  # GitHub CLI compatible
    "GITHUB_TOKEN",  # GitHub Actions compatible
]


class CopilotSetupError(RuntimeError):
    """Raised when the Copilot SDK environment is not correctly configured."""


def _resolve_copilot_cli_path() -> Optional[str]:
    """Find the executable copy of the copilot CLI created by copy_executables.

    rules_python strips the executable bit from binaries inside wheels.
    The pip.whl_mods / copy_executables mechanism creates an executable
    copy called ``copilot_cli`` next to the package.  We walk up from
    ``copilot.__file__`` until we find it.

    IMPORTANT: we must NOT resolve symlinks (Path.resolve()) because in
    the Bazel runfiles tree the symlinks point back to the source repo
    where the genrule output does not exist.  The raw __file__ path
    stays inside the execution root where the copy IS present.
    """
    import copilot as _copilot_pkg

    pkg_file = Path(_copilot_pkg.__file__)  # .../site-packages/copilot/__init__.py
    # Walk up: copilot/ -> site-packages/ -> lib/ -> ... -> repo root
    current = pkg_file.parent
    for _ in range(10):
        candidate = current / "copilot_cli"
        if candidate.exists():
            return str(candidate)
        current = current.parent
    return None


def _check_cli_binary(cli_path: str) -> list[str]:
    """Validate that the CLI binary exists and is executable.

    Returns a list of problem descriptions (empty = all good).
    """
    problems: list[str] = []
    p = Path(cli_path)
    if not p.exists():
        problems.append(f"Copilot CLI binary not found at: {cli_path}")
        return problems
    if not p.is_file():
        problems.append(f"Copilot CLI path is not a file: {cli_path}")
        return problems
    mode = p.stat().st_mode
    if not (mode & stat.S_IXUSR):
        problems.append(
            f"Copilot CLI binary is NOT executable (mode {oct(mode)}): {cli_path}\n"
            "  Hint: rules_python strips +x from wheel binaries. Make sure\n"
            "  pip.whl_mods / copy_executables is configured in MODULE.bazel."
        )
    return problems


def _check_environment() -> list[str]:
    """Check that the runtime environment has what the Copilot CLI needs.

    Returns a list of problem descriptions (empty = all good).
    """
    problems: list[str] = []

    if not os.environ.get("HOME"):
        problems.append(
            "HOME environment variable is not set.\n"
            "  The Copilot CLI needs HOME to locate stored OAuth credentials.\n"
            "  Ensure .bazelrc.ai_checker contains:  build --action_env=HOME"
        )

    # The Copilot CLI binary (Node.js) uses fetch() to reach api.github.com.
    # Behind a corporate proxy it needs HTTPS_PROXY.
    if not os.environ.get("HTTPS_PROXY") and not os.environ.get("https_proxy"):
        problems.append(
            "HTTPS_PROXY / https_proxy environment variable is not set.\n"
            "  If you are behind a corporate proxy the Copilot CLI cannot\n"
            "  reach api.github.com and will fail with 'TypeError: fetch failed'.\n"
            "  Ensure .bazelrc.ai_checker contains:  build --action_env=HTTPS_PROXY"
        )

    return problems


def _describe_auth_sources() -> str:
    """Return a human-readable summary of available auth sources."""
    lines = ["Authentication sources detected:"]
    found_any = False

    for var in _AUTH_ENV_VARS:
        val = os.environ.get(var)
        if val:
            # Mask the token for security
            masked = val[:4] + "..." + val[-4:] if len(val) > 10 else "****"
            lines.append(f"  [OK] ${var} = {masked}")
            found_any = True
        else:
            lines.append(f"  [  ] ${var} — not set")

    home = os.environ.get("HOME", "")
    if home:
        lines.append(f"  [OK] $HOME = {home}  (CLI can search system keychain)")
    else:
        lines.append(
            "  [  ] $HOME — not set  (CLI cannot find stored OAuth credentials)"
        )

    if not found_any and not home:
        lines.append("")
        lines.append("  ** No authentication source available! **")
        lines.append(
            "  Fix: set COPILOT_GITHUB_TOKEN, or ensure HOME is passed to the action."
        )
        lines.append(
            "  See: https://github.com/github/copilot-sdk/blob/main/docs/auth/index.md"
        )

    # Network / proxy info
    lines.append("")
    proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy")
    if proxy:
        lines.append(f"  [OK] HTTPS_PROXY = {proxy}")
    else:
        lines.append(
            "  [  ] HTTPS_PROXY — not set  (may cause 'fetch failed' behind a proxy)"
        )

    return "\n".join(lines)


from langchain_core.callbacks import (
    CallbackManagerForLLMRun,
    AsyncCallbackManagerForLLMRun,
)
from langchain_core.language_models.chat_models import BaseChatModel
from langchain_core.messages import (
    AIMessage,
    BaseMessage,
    HumanMessage,
    SystemMessage,
    ToolMessage,
)
from langchain_core.outputs import ChatGeneration, ChatResult
from langchain_core.tools import BaseTool
from langchain_core.utils.function_calling import convert_to_openai_tool
from pydantic import BaseModel, Field, PrivateAttr


def _convert_tools_to_openai_format(
    tools: Sequence[dict[str, Any] | type | Callable | BaseTool],
) -> list[dict[str, Any]]:
    """Convert LangChain tool specs to OpenAI-format tool definitions."""
    result = []
    for tool in tools:
        if isinstance(tool, dict):
            # Already a dict — assume it's in OpenAI format or close enough
            result.append(tool)
        else:
            result.append(convert_to_openai_tool(tool))
    return result


def _build_copilot_tools(
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

        # Capture loop variables explicitly to avoid the closure-over-loop-variable
        # pitfall.  Although _noop_handler is never actually invoked (tool
        # execution is intercepted at the LangChain level), the correct capture
        # pattern is important for correctness and future maintainability.
        def _make_noop_handler(tool_name: str):
            async def _noop_handler(invocation: ToolInvocation) -> ToolResult:
                # This handler should never actually be invoked because we
                # intercept tool requests at the LangChain level.
                return ToolResult(
                    textResultForLlm="Tool execution is managed by LangChain.",
                    resultType="success",
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


def _deep_decode_json_strings(obj: Any) -> Any:
    """Recursively decode values that are JSON-encoded strings.

    Some LLMs (e.g. Claude via the Copilot SDK) double-encode nested
    lists or objects as JSON strings inside the outer tool-call arguments
    dict.  This function walks the structure and replaces any string value
    that successfully parses as a JSON array or object with the decoded
    Python value, leaving plain strings untouched.
    """
    if isinstance(obj, dict):
        return {k: _deep_decode_json_strings(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [_deep_decode_json_strings(v) for v in obj]
    if isinstance(obj, str):
        stripped = obj.strip()
        if stripped and stripped[0] in ("{", "["):
            try:
                decoded = json.loads(stripped)
                # Only substitute if the result is a richer structure
                if isinstance(decoded, (dict, list)):
                    return _deep_decode_json_strings(decoded)
            except (json.JSONDecodeError, ValueError):
                pass
    return obj


def _messages_to_prompt(messages: list[BaseMessage]) -> str:
    """Convert a list of LangChain messages into a single prompt string.

    The Copilot SDK accepts a plain text prompt rather than a structured
    message array. We serialise the conversation into a tagged format so
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
            # Include any tool calls the AI made previously
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


def _extract_system_message(messages: list[BaseMessage]) -> Optional[str]:
    """Extract the system message content if the first message is a SystemMessage."""
    if messages and isinstance(messages[0], SystemMessage):
        content = messages[0].content
        return content if isinstance(content, str) else json.dumps(content)
    return None


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
    _client: Optional[CopilotClient] = PrivateAttr(default=None)
    _client_started: bool = PrivateAttr(default=False)
    _bound_tools: list[dict[str, Any]] = PrivateAttr(default_factory=list)
    _tool_choice: Optional[str] = PrivateAttr(default=None)
    _ls_structured_output_format: Optional[dict[str, Any]] = PrivateAttr(default=None)

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
    # Client lifecycle
    # ------------------------------------------------------------------ #

    async def _ensure_client(self) -> CopilotClient:
        """Lazily create, start, and verify the CopilotClient.

        Performs pre-flight checks before starting the CLI:
        1. Resolves the CLI binary path (copy_executables workaround)
        2. Validates the binary exists and is executable
        3. Checks required environment variables (HOME, token vars)
        4. Starts the CLI server
        5. Verifies authentication via ``get_auth_status()``

        Raises:
            CopilotSetupError: If any pre-flight check fails with a
                detailed, actionable error message.
        """
        if self._client is None:
            opts = dict(self.copilot_client_options or {})

            # --- Resolve CLI binary path --------------------------------
            if "cli_path" not in opts and "cli_url" not in opts:
                resolved = _resolve_copilot_cli_path()
                if resolved:
                    opts["cli_path"] = resolved
                    logger.info("Resolved Copilot CLI path: %s", resolved)
                else:
                    logger.warning(
                        "Could not find copilot_cli (copy_executables target). "
                        "Falling back to bundled binary — this may fail with "
                        "PermissionError if the executable bit was stripped."
                    )

            # --- Pre-flight: check binary -------------------------------
            cli_path = opts.get("cli_path")
            if cli_path:
                problems = _check_cli_binary(cli_path)
                if problems:
                    raise CopilotSetupError(
                        "Copilot CLI binary check failed:\n"
                        + "\n".join(f"  - {p}" for p in problems)
                    )

            # --- Pre-flight: check environment --------------------------
            env_problems = _check_environment()
            if env_problems:
                logger.warning(
                    "Environment issues detected:\n%s\n%s",
                    "\n".join(f"  - {p}" for p in env_problems),
                    _describe_auth_sources(),
                )
                # Don't hard-fail here — the user may have a token env var.
                # We'll verify auth after starting the client.

            logger.info("Starting CopilotClient...\n%s", _describe_auth_sources())
            self._client = CopilotClient(opts or None)

        if not self._client_started:
            try:
                await self._client.start()
            except PermissionError as exc:
                raise CopilotSetupError(
                    f"PermissionError starting Copilot CLI: {exc}\n"
                    "  The CLI binary is not executable. Make sure\n"
                    "  pip.whl_mods / copy_executables is configured in MODULE.bazel\n"
                    "  to create an executable copy of copilot/bin/copilot."
                ) from exc
            except RuntimeError as exc:
                if "timeout" in str(exc).lower() or "Timeout" in str(exc):
                    raise CopilotSetupError(
                        f"Timeout starting Copilot CLI server: {exc}\n"
                        "  The CLI started but did not become ready in time.\n"
                        "  This usually means the CLI cannot authenticate.\n\n"
                        + _describe_auth_sources()
                        + "\n\n"
                        "  Possible fixes:\n"
                        "  1. Run 'copilot' in a terminal and sign in interactively.\n"
                        "  2. Set COPILOT_GITHUB_TOKEN (or GH_TOKEN / GITHUB_TOKEN)\n"
                        "     and pass it via --action_env=COPILOT_GITHUB_TOKEN.\n"
                        "  3. Ensure HOME is available in the action environment\n"
                        "     (use_default_shell_env = True in the Bazel rule).\n"
                        "  See: https://github.com/github/copilot-sdk/blob/main/docs/auth/index.md"
                    ) from exc
                raise
            except Exception as exc:
                raise CopilotSetupError(
                    f"Failed to start CopilotClient: {type(exc).__name__}: {exc}\n\n"
                    + _describe_auth_sources()
                ) from exc

            self._client_started = True

            # --- Post-start: verify authentication ----------------------
            try:
                auth_status = await self._client.get_auth_status()
                if (
                    hasattr(auth_status, "isAuthenticated")
                    and auth_status.isAuthenticated
                ):
                    user = getattr(auth_status, "login", "unknown")
                    logger.info("Copilot authenticated as: %s", user)
                elif (
                    hasattr(auth_status, "is_authenticated")
                    and auth_status.is_authenticated
                ):
                    user = getattr(auth_status, "login", "unknown")
                    logger.info("Copilot authenticated as: %s", user)
                else:
                    raise CopilotSetupError(
                        "Copilot CLI started but is NOT authenticated.\n"
                        f"  Auth status: {auth_status}\n\n"
                        + _describe_auth_sources()
                        + "\n\n"
                        "  Possible fixes:\n"
                        "  1. Run 'copilot' in a terminal and sign in interactively.\n"
                        "  2. Set COPILOT_GITHUB_TOKEN (or GH_TOKEN / GITHUB_TOKEN).\n"
                        "  See: https://github.com/github/copilot-sdk/blob/main/docs/auth/index.md"
                    )
            except CopilotSetupError:
                raise
            except Exception as exc:
                # get_auth_status itself failed — log but don't block.
                # The actual LLM call will fail with a clearer error if auth
                # is truly broken.
                logger.warning(
                    "Could not verify auth status (non-fatal): %s: %s",
                    type(exc).__name__,
                    exc,
                )

        return self._client

    async def aclose(self) -> None:
        """Shut down the underlying Copilot CLI process."""
        if self._client and self._client_started:
            await self._client.stop()
            self._client_started = False

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
        openai_tools = _convert_tools_to_openai_format(tools)
        # Create a shallow copy with the tools attached
        new = self.model_copy()
        new._bound_tools = openai_tools
        new._tool_choice = tool_choice
        new._ls_structured_output_format = kwargs.get("ls_structured_output_format")
        new._client = self._client
        new._client_started = self._client_started
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
            client = await self._ensure_client()
        except CopilotSetupError:
            raise  # Already has a clear message
        except Exception as exc:
            raise CopilotSetupError(
                f"Unexpected error initialising Copilot SDK: {type(exc).__name__}: {exc}\n\n"
                + _describe_auth_sources()
            ) from exc

        # Build session config
        session_config: SessionConfig = {
            "model": kwargs.get("model", self.model),
        }

        # Disable all built-in tools so only our bound tools are available
        session_config["available_tools"] = []

        # Merge any extra tools from kwargs with bound tools
        extra_tools = kwargs.get("tools", [])
        all_openai_tools = self._bound_tools + (
            _convert_tools_to_openai_format(extra_tools) if extra_tools else []
        )

        if all_openai_tools:
            session_config["tools"] = _build_copilot_tools(all_openai_tools)

        # Use system message from the conversation if present
        system_content = _extract_system_message(messages)
        if system_content:
            base_system = system_content
            # Remove system message from prompt construction
            prompt_messages = [m for m in messages if not isinstance(m, SystemMessage)]
        else:
            base_system = "You are a helpful assistant."
            prompt_messages = messages

        # When tool_choice="any" (structured output), force the model to
        # respond exclusively via tool calls.
        if self._tool_choice == "any" and all_openai_tools:
            tool_names = [t.get("function", t)["name"] for t in all_openai_tools]
            base_system += (
                "\n\nIMPORTANT: You MUST respond by calling one of the following "
                f"tools: {', '.join(tool_names)}. "
                "Do NOT respond with plain text. You MUST use a tool call. "
                "Pass your entire answer as arguments to the tool."
            )

        session_config["system_message"] = {
            "mode": "replace",
            "content": base_system,
        }

        # Disable infinite sessions for simple request/response
        session_config["infinite_sessions"] = {"enabled": False}

        # Create session
        session = await client.create_session(session_config)

        try:
            # Build the prompt from messages
            prompt = _messages_to_prompt(prompt_messages)

            # Collect streaming events for tool calls
            tool_requests: list[Any] = []

            def _event_handler(event: Any) -> None:
                if event.type == SessionEventType.ASSISTANT_MESSAGE:
                    if event.data.tool_requests:
                        tool_requests.extend(event.data.tool_requests)

            unsubscribe = session.on(_event_handler)

            try:
                response = await session.send_and_wait(
                    {"prompt": prompt},
                    timeout=self.timeout,
                )
            finally:
                unsubscribe()

            # Extract content
            content = ""
            if response and response.data and response.data.content:
                content = response.data.content

            # Check for tool requests on the response itself
            if response and response.data and response.data.tool_requests:
                for tr in response.data.tool_requests:
                    if tr not in tool_requests:
                        tool_requests.append(tr)

            # Build tool_calls for the AIMessage
            tool_calls = []
            for tr in tool_requests:
                args = tr.arguments
                if isinstance(args, str):
                    try:
                        args = json.loads(args)
                    except (json.JSONDecodeError, TypeError):
                        args = {"raw": args}
                elif args is None:
                    args = {}

                # Deep-decode: some models (e.g. Claude via Copilot SDK) return
                # nested lists/objects as JSON-encoded strings inside the outer
                # tool-call arguments dict.  Un-double-encode them so LangChain's
                # structured-output parser receives proper Python objects.
                if isinstance(args, dict):
                    args = _deep_decode_json_strings(args)

                tool_calls.append(
                    {
                        "name": tr.name,
                        "args": args if isinstance(args, dict) else {"raw": args},
                        "id": tr.tool_call_id,
                    }
                )

            # Build the AIMessage
            ai_message = AIMessage(
                content=content,
                tool_calls=tool_calls if tool_calls else [],
                response_metadata={
                    "model": self.model,
                },
            )

            return ChatResult(
                generations=[ChatGeneration(message=ai_message)],
            )
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
            # We're already in an async context — use a helper to run in
            # a new thread to avoid blocking the event loop.
            import concurrent.futures

            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                future = pool.submit(
                    asyncio.run,
                    self._agenerate(messages, stop, None, **kwargs),
                )
                return future.result()
        else:
            return asyncio.run(self._agenerate(messages, stop, None, **kwargs))
