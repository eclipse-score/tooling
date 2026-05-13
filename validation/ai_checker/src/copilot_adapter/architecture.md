<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# copilot_adapter — Architecture

## Overview

`copilot_adapter` is a [LangChain](https://python.langchain.com/) integration layer that
bridges the **GitHub Copilot SDK** (`github-copilot-sdk`) to the LangChain ecosystem.
It exposes a single public class, `ChatCopilot`, which is a drop-in replacement for any
other LangChain `BaseChatModel` (e.g. `ChatOpenAI`).

The adapter translates between two different worlds:

| LangChain side | Copilot SDK side |
|---|---|
| `list[BaseMessage]` (typed message objects) | A single plain-text prompt string with role tags |
| `SystemMessage` | `SessionConfig.system_message` (injected once per session) |
| `BaseTool` / OpenAI tool dict | `copilot.tools.Tool` with async handler |
| Pydantic `BaseModel` schema | JSON schema embedded in the system prompt |
| `AIMessage` with `tool_calls` | `ExternalToolRequestedData` broadcast events |

---

## Component Diagram



---

## Module Responsibilities

### `copilot_langchain.py` — `ChatCopilot`

The central public class. Inherits from LangChain's `BaseChatModel` so it can be used
anywhere a standard LangChain model is expected.

| Method | Role |
|---|---|
| `with_structured_output(schema)` | Returns a composed `Runnable` chain for structured JSON output (see below) |
| `bind_tools(tools)` | Returns a new `ChatCopilot` instance with the given tools pre-registered |
| `_agenerate(messages)` | **Async core** — creates a Copilot session, sends the prompt, collects the response |
| `_generate(messages)` | Sync bridge: runs `_agenerate` in a thread-pool executor if an event loop is already running |

### `_client_manager.py` — `CopilotClientManager`

Owns the lifecycle of the single `CopilotClient` / CLI subprocess. The same subprocess
is reused across calls (cached in `_client`).

Pre-flight sequence executed once before the first request:
1. Resolve the `copilot_cli` binary path (Bazel `copy_executables` workaround)
2. Verify the binary exists and is executable
3. Hard-fail if no auth source is found (`COPILOT_GITHUB_TOKEN`, `GH_TOKEN`, `GITHUB_TOKEN`, or `~/.copilot/config.json`)
4. Warn (non-fatal) about missing `$HOME` or `HTTPS_PROXY`
5. Spawn the subprocess and authenticate via `get_auth_status()`

### `_message_converter.py`

Converts the LangChain message list into the formats the Copilot SDK accepts.

- **`extract_system_message(messages)`** — Pulls out the first `SystemMessage` and
  returns its string content. This is passed as `SessionConfig.system_message` so the
  Copilot CLI handles it as a true system prompt (not just prepended text).

- **`messages_to_prompt(messages)`** — Serialises all remaining messages into a single
  tagged plain-text string:
  ```
  [user]
  What is 2 + 2?

  [assistant]
  4
  [tool_call id=abc name=add]
  {"a": 2, "b": 2}

  [tool_result id=abc]
  4
  ```

### `_tool_converter.py`

Converts tool definitions between three representations:

```
LangChain BaseTool / Callable / type
         │
         ▼ convert_to_openai_tool()
OpenAI function dict  {"type": "function", "function": {"name": ..., "parameters": ...}}
         │
         ▼ build_copilot_tools()
copilot.tools.Tool   (with async no-op handler)
```

The handler is always a no-op because when using `with_structured_output` tool execution
is bypassed entirely; when using `bind_tools` directly, tool execution is managed by the
LangChain agent loop, not by the Copilot CLI.

`deep_decode_json_strings` recursively unwraps values that the model has
double-encoded as JSON strings inside the outer tool-call arguments dict.

### `_preflight.py`

Stateless helper functions called by `CopilotClientManager` before startup:

- `resolve_copilot_cli_path()` — walks up from `copilot.__file__` to find the
  `copilot_cli` binary that Bazel's `copy_executables` placed next to the package.
- `check_cli_binary(path)` — checks existence and executable bit.
- `check_auth_sources()` — scans env vars and `~/.copilot/config.json`; hard-fails if
  none are present.
- `check_environment()` — warns about missing `HOME`, `HTTPS_PROXY`, or proxy vars.
- `describe_auth_sources()` — formats a human-readable description for error messages.

### `_errors.py`

- `CopilotSetupError` — a `RuntimeError` subclass raised for any configuration or
  startup failure. Carries an actionable message for the user.
- `AUTH_ENV_VARS` — ordered list of accepted auth environment variables.

---

## Data Flow: `with_structured_output`

The primary usage path (used by `ai_checker_core`). `with_structured_output(schema)`
returns a composed chain: `_inject | ChatCopilot | _parse`.

1. **`_inject`** — appends the Pydantic schema (serialised to JSON) to the
   `SystemMessage`, instructing the model to respond with only a matching JSON object.
2. **`ChatCopilot._agenerate`** — extracts the system message into
   `SessionConfig.system_message`, serialises remaining messages into a role-tagged
   plain-text prompt, creates a Copilot session, and calls `send_and_wait`. Returns an
   `AIMessage` whose `.content` is the model's raw text reply.
3. **`_parse`** — strips any markdown fences, extracts the outermost `{…}` substring,
   parses it with `json.loads`, and validates it with `schema.model_validate`. On any
   failure it raises a `ValueError` containing the full raw LLM output and the specific
   error (JSON byte position or Pydantic field detail).

> **Why JSON-in-prompt instead of tool calling?**
> The Copilot CLI uses a Claude model with reasoning enabled. That model ignores
> `tool_choice="any"` and always responds in plain text. Embedding the JSON schema
> directly in the system prompt is reliably followed.

---

## Data Flow: `bind_tools` (direct tool use)

Used when the LangChain agent loop — not the adapter — executes tools.

1. `bind_tools(tools)` converts each tool to OpenAI format, then to a `copilot.tools.Tool`
   with a no-op handler, and registers them in `SessionConfig`.
2. Tool calls arrive as `ExternalToolRequestedData` broadcast events (SDK protocol v3)
   or in `AssistantMessageData.tool_requests` (legacy fallback).
3. Both sources are merged, deduplicated by `tool_call_id`, and returned as
   `AIMessage.tool_calls` for the LangChain agent loop to dispatch.

---

## Authentication

The Copilot CLI requires a valid GitHub OAuth token to contact the Copilot API.
`_preflight.py` checks the following sources in priority order during pre-flight:

| Priority | Source | Notes |
|---|---|---|
| 1 | `COPILOT_GITHUB_TOKEN` env var | Recommended for explicit Copilot usage |
| 2 | `GH_TOKEN` env var | GitHub CLI compatible |
| 3 | `GITHUB_TOKEN` env var | GitHub Actions compatible |
| 4 | `~/.copilot/config.json` | Written by `gh copilot` interactive login |

If **none** of these sources is present, `CopilotClientManager` raises a
`CopilotSetupError` before spawning the subprocess (hard fail — no point starting
the CLI without credentials).

If at least one source exists, the CLI is started and `get_auth_status()` is called to
confirm the token is accepted by the GitHub API. A failed status also raises
`CopilotSetupError` with an actionable message.

### Bazel / headless environments

In the CI/Bazel setup the `--config=copilot` bazelrc flag forwards `HOME` and the
proxy environment variables (`HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`) to the action
sandbox via `--action_env`. Without `HOME` the CLI cannot locate `~/.copilot/config.json`,
so a token env var must be set instead. `_preflight.py` emits a warning (non-fatal)
when `HOME` is unset.

---

## Error Handling Summary

| Failure point | Exception type | What is logged |
|---|---|---|
| CLI binary missing / not executable | `CopilotSetupError` | Path checked, alternatives suggested |
| No auth source found | `CopilotSetupError` | Lists all env vars and config file path |
| Copilot SDK startup error | `CopilotSetupError` | Wraps original exception + auth description |
| Model returns no JSON object | `ValueError` | Full LLM output |
| Model returns malformed JSON | `ValueError` | `json.JSONDecodeError` position + full LLM output |
| Model returns wrong JSON structure | `ValueError` | Pydantic field-level `ValidationError` + full LLM output |
