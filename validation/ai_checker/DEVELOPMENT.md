<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# AI Checker — Development Guide

Technical reference for developing and extending the AI Checker. For usage and
Bazel integration, see [README.md](README.md).

## Architecture

The AI Checker is a single `ai_checker` package under the `src/` import root,
clustered by responsibility:

| Path | Purpose |
|------|---------|
| `src/ai_checker/` | Core analysis framework (orchestrator, scoring, caching, reporting, guidelines reader) + the `AnalysisAgent` interface. **No AI-SDK / LangChain dependency.** |
| `src/ai_checker/extractors/` | `ArtefactExtractor` implementations: `base.py` (ABC), `requirement_extractor.py` (TRLC), `architecture_extractor.py` (raw PlantUML). |
| `src/ai_checker/agents/` | AI backends: `CopilotAgent` (default, direct Copilot SDK) and the optional `LangChainAgent` (wraps any LangChain `BaseChatModel`), plus Copilot session plumbing (`_client_manager`, `_preflight`, `_errors`). |

The core never talks to an SDK directly; it depends on the `AnalysisAgent`
interface (`analyze(system_prompt, artefacts_text) -> AnalysisResults`).
`CopilotAgent` is the default implementation; `LangChainAgent` wraps any
LangChain `BaseChatModel` for custom backends.

### Diagrams

**Deployment overview:**

![Deployment Diagram](_assets/deployment_diagram.svg)

**Class relationships:**

![Class Diagram](_assets/class_diagram.svg)

## Key Components

### `AIChecker` (`src/ai_checker/ai_checker_core.py`)

Performs the async AI analysis. Responsibilities:

- Splits artefacts into batches (by count via `--batch-size` and by total
  character length via `--max-batch-chars`)
- Processes batches concurrently, rate-limited by an `asyncio.Semaphore`
- Calls `AnalysisAgent.analyze(system_prompt, artefacts_text)` per batch
- Manages the optional result cache (`AnalysisCache`)

### `AnalysisAgent` (`src/ai_checker/analysis_agent.py`)

The interface between the core and any AI backend. One async method,
`analyze(system_prompt, artefacts_text) -> AnalysisResults`, plus a
`get_usage() -> Usage` accessor (backends call the protected `_record_usage()`
to accumulate tokens / cost / AI credits) and an `aclose()` hook for releasing
resources. Keeps the core free of any SDK / LangChain dependency.

### `AnalysisOrchestrator` (`src/ai_checker/orchestrator.py`)

Top-level coordinator. Selects the extractor, builds the agent, assembles the
system prompt, runs the AI checker, and exposes the CLI entry point (`main()`).

The system prompt is layered in this order:

1. **General + type guidelines** — concatenated from the `--guidelines`
   directory (e.g. `general.md` + the element-type guideline).
2. **Project-specific guidelines** — graded rules from `--project-guidelines`,
   appended under a `# PROJECT-SPECIFIC GUIDELINES` heading.
3. **Background context** — read-only reference material from `--context-file`,
   appended under a clearly labelled "reference only — not graded" heading.

### `GuidelinesReader` (`src/ai_checker/guidelines_reader.py`)

Reads text documents into a `name -> content` mapping, from either a directory
(guidelines, `*.md`) or an explicit file list (project guidelines / background
context, `.md` / `.puml`), filtered by extension.

### `RequirementExtractor` / `ArchitectureExtractor` (`src/ai_checker/extractors/`)

`ArtefactExtractor` implementations selected by `--artefact-type`.
`RequirementExtractor` parses TRLC files via the TRLC Python API (only objects
under `--input` are analyzed; `--deps` are loaded for link resolution).
`ArchitectureExtractor` reads the raw `.puml` source of architecture diagrams.
Both return `dict[str, dict[str, Any]]`.

### Reports (`src/ai_checker/reports/`)

`ResultFormatter` (`reports/formatter.py`) builds **one** `AnalysisReport`
(`reports/models.py`) in memory — report metadata, guideline texts, and the
per-artefact analyses — then renders the requested format via a
`ReportRenderer` (`reports/base.py`), chosen by output extension:

| Extension | Renderer | Notes |
|-----------|----------|-------|
| `.json` (default) | `JsonRenderer` | Self-contained envelope; keeps top-level `analyses`. |
| `.html` | `HtmlRenderer` | Styled page + per-guideline `.md` subpages. |
| `.rst` | `RstRenderer` | Standalone reStructuredText + per-guideline `.rst` subpages. |

Shared helpers live in `reports/text_utils.py` (slugs, severity, markdown→HTML)
and `reports/metadata.py` (git hash, timestamp). Add a format by subclassing
`ReportRenderer` and registering it in `reports/formatter.py`.

## Agents

The `ai_checker.agents` package provides AI backends. The core depends only on
the `AnalysisAgent` interface; this package supplies two implementations:

| Class | Path | Role |
|---|---|---|
| **`CopilotAgent`** | `copilot_agent.py` | **Default.** Talks to the **GitHub Copilot SDK** directly — no LangChain. Owns one CLI session per request, embeds the JSON schema in the system prompt, parses the reply into `AnalysisResults`. |
| `LangChainAgent` | `langchain_agent.py` | Optional. Wraps any LangChain `BaseChatModel` (e.g. `ChatOpenAI`) as an `AnalysisAgent`, via `with_structured_output(AnalysisResults)`. Used when a custom model is supplied. |

The two backends are independent: `CopilotAgent` needs only the Copilot SDK,
`LangChainAgent` needs only `langchain_core`. The Copilot session plumbing
(`_client_manager`, `_preflight`, `_errors`) is used by `CopilotAgent` only.

### Module Responsibilities

#### `copilot_agent.py` — `CopilotAgent` (default)

Implements `AnalysisAgent.analyze(system_prompt, artefacts_text) -> AnalysisResults`
directly against the Copilot SDK:

1. `CopilotClientManager.ensure_client()` (shared pre-flight + session plumbing).
2. Build the system message = `system_prompt` + a JSON-schema instruction for
   `AnalysisResults`; send `artefacts_text` as the prompt.
3. Parse the model's text reply into `AnalysisResults` (`_extract_json_object`
   scans for the first balanced `{...}` object, tolerating fences/prose, then
   validates against the schema).
4. Usage capture: subscribe to the session's `assistant.usage` events and
   accumulate the typed `AssistantUsageData` fields (input/output tokens,
   experimental `cost`, and `copilotUsage.totalNanoAiu` for AI credits) into a
   `Usage`, recorded via `_record_usage()`.

#### `langchain_agent.py` — `LangChainAgent` (optional)

Adapts any LangChain `BaseChatModel` to `AnalysisAgent` via
`model.with_structured_output(AnalysisResults).ainvoke([SystemMessage, HumanMessage])`.
Only used when a custom `create_agent()` returns one, e.g.:

```python
from ai_checker.agents.langchain_agent import LangChainAgent
from langchain_openai import ChatOpenAI

def create_agent(model_name):
    return LangChainAgent(ChatOpenAI(model=model_name))
```

#### `_client_manager.py` — `CopilotClientManager`

Owns the lifecycle of the single `CopilotClient` / CLI subprocess. The same
subprocess is reused across calls. Pre-flight sequence executed once before the
first request:

1. Resolve the `copilot_cli` binary path (Bazel `copy_executables` workaround).
2. Verify the binary exists and is executable.
3. Hard-fail if no auth source is found (`COPILOT_GITHUB_TOKEN`, `GH_TOKEN`,
   `GITHUB_TOKEN`, or `~/.copilot/config.json`).
4. Warn (non-fatal) about missing `$HOME` or `HTTPS_PROXY`.
5. Spawn the subprocess and authenticate via `get_auth_status()`.

#### `_preflight.py`

Stateless helpers called by `CopilotClientManager` before startup:
`resolve_copilot_cli_path()`, `check_cli_binary()`, `check_auth_sources()`,
`check_environment()`, `describe_auth_sources()`.

#### `_errors.py`

- `CopilotSetupError` — a `RuntimeError` subclass raised for any configuration
  or startup failure. Carries an actionable message for the user.
- `AUTH_ENV_VARS` — ordered list of accepted auth environment variables.

### Why JSON-in-prompt instead of structured output / tool calling?

The GitHub Copilot SDK (v0.3.0) does **not** expose any native
structured-output mechanism: `SessionConfig` has no `response_format`,
`output_schema`, or `json_schema` field, and `send_and_wait` returns plain text
in `response.data.content`.

The SDK does expose a `tools` (function-calling) mechanism, but it provides **no
benefit** for forcing a structured result here:

- The SDK's `Tool` is `{name, description, handler, parameters}` and nothing in
  the SDK constrains the generated tool arguments to the `parameters` schema, so
  any tool-call payload would still have to be validated manually — exactly like
  the text reply today.
- The Copilot CLI's Claude model (reasoning enabled) ignores tool-calling
  instructions and always replies in plain text, so registering a tool yields no
  tool calls.
- Tool calling is designed for *agentic actions* (fetching data, running code),
  not for output formatting; we only need a single structured result.

Therefore `CopilotAgent` embeds the `AnalysisResults` JSON schema in the system
prompt and parses the reply (`_extract_json_object` + `model_validate`). The
`LangChainAgent` path achieves structured output natively via the wrapped
model's `with_structured_output(AnalysisResults)`.

### Authentication

The Copilot CLI requires a valid GitHub OAuth token. `_preflight.py` checks the
following sources in priority order during pre-flight:

| Priority | Source | Notes |
|---|---|---|
| 1 | `COPILOT_GITHUB_TOKEN` env var | Recommended for explicit Copilot usage |
| 2 | `GH_TOKEN` env var | GitHub CLI compatible |
| 3 | `GITHUB_TOKEN` env var | GitHub Actions compatible |
| 4 | `~/.copilot/config.json` | Written by `gh copilot` interactive login |

If **none** is present, `CopilotClientManager` raises a `CopilotSetupError`
before spawning the subprocess. If at least one exists, the CLI is started and
`get_auth_status()` confirms the token is accepted.

In the Bazel setup the analysis runs at **test time** (see
[Bazel Test Rules](#bazel-test-rules)). The test rules bake the required
environment-variable inheritance into each target via `RunEnvironmentInfo`
(`inherited_environment`), so `HOME`, the GitHub tokens, and the proxy variables
are inherited from the invoking shell without any `--config=copilot` /
`--test_env` flag. `HOME` is essential because the test runner otherwise resets
it to `$TEST_TMPDIR`, hiding `~/.copilot/config.json`.

### Error Handling Summary

| Failure point | Exception type | What is logged |
|---|---|---|
| CLI binary missing / not executable | `CopilotSetupError` | Path checked, alternatives suggested |
| No auth source found | `CopilotSetupError` | Lists all env vars and config file path |
| Copilot SDK startup error | `CopilotSetupError` | Wraps original exception + auth description |
| Model returns no JSON object | `ValueError` | Full LLM output |
| Model returns malformed JSON | `ValueError` | `json.JSONDecodeError` position + full LLM output |
| Model returns wrong JSON structure | `ValueError` | Pydantic field-level `ValidationError` + full LLM output |

## Caching Design

`AnalysisCache` keys results by `SHA-256(artefacts_json + guidelines + model_name)`.
It is **only** usable via the CLI `--cache` flag. The Bazel test rules
deliberately omit `--cache`; the AI tests are tagged `external` (their result is
non-deterministic), so Bazel never caches them and a stale cache cannot mask a
real regression.

## Bazel Test Rules

The `trlc_requirements_ai_test` / `architecture_ai_test` macros wrap private
rules (`_trlc_requirements_ai_test` / `_architecture_ai_test`) that run the AI
analysis **at test time**, not as a build action. Rationale:

- The AI call is inherently non-hermetic (outbound network + user credentials).
  Tests are the idiomatic home for non-hermetic work, and environment
  inheritance is baked into the target via `RunEnvironmentInfo` — cleaner than
  `--action_env` on a build action and requires no `--config` flag.
- Reports are written to `$TEST_UNDECLARED_OUTPUTS_DIR`, so they are captured in
  `bazel-testlogs/.../test.outputs/outputs.zip` instead of `bazel-bin`.

Flow:

1. `_run_ai_analysis` (`ai_checker.bzl`) bakes the orchestrator arguments
   (runfiles-relative `short_path`s) and writes a small launcher inline via
   `ctx.actions.write`. Guideline files are passed individually with
   `--guidelines-file` (the default guideline sets span several directories, so
   a single derived directory would drop some); the requirement grading scope
   is defined by the explicit `--req-file`s rather than a derived directory.
2. Per the [Bazel Test Encyclopedia](https://bazel.build/reference/test-encyclopedia),
   a test starts with its working directory at `$TEST_SRCDIR/$TEST_WORKSPACE`
   (the runfiles root), so the launcher needs no runfiles probing: it simply
   `exec`s the orchestrator `py_binary` with the baked arguments and
   `--score-threshold`.
3. The orchestrator detects `$TEST_UNDECLARED_OUTPUTS_DIR` and writes all of its
   reports (`analysis.json` / `.html` / `.rst`, the guideline pages and
   `debug.log`) there itself, so the launcher carries no output-path plumbing.
4. The orchestrator runs the analysis and, when `--score-threshold` is set,
   computes the average score and exits non-zero if it is below the threshold —
   so a failing score fails the test directly (no separate checker script).

The macros inject default tags (`no-sandbox`, `requires-network`, `external`)
and merge any caller-supplied `tags`. Environment inheritance is provided by
`RunEnvironmentInfo(inherited_environment = [...])`.

## Adding a New Artefact Type

1. Subclass `ArtefactExtractor` (`src/ai_checker/extractors/base.py`) and
   implement `extract() -> dict[str, dict[str, Any]]`.
2. Add a new `--artefact-type` value and select your extractor in
   `AnalysisOrchestrator.analyze_directory()`.
3. Add a corresponding Bazel rule in `ai_checker.bzl` following the pattern of
   `_trlc_requirements_ai_test_impl` / `_architecture_ai_test_impl`.

## Adding a Custom AI Backend

Provide a `create_agent(model_name) -> AnalysisAgent` factory in an
`ai_model.py` file and wire it via the `_custom_ai_model` attribute (see
[README.md](README.md)). The agent must implement:

```python
async def analyze(self, system_prompt: str, artefacts_text: str) -> AnalysisResults
```

To reuse a LangChain model, return the bundled `LangChainAgent` wrapper.

## Updating Python Dependencies

```bash
# Core + Copilot SDK dependencies
bazel run //validation/ai_checker:requirements.update
```
