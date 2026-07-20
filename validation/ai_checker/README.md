<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# AI Checker

AI-powered analysis tool for engineering artefacts against guidelines.

---

## User Guide

### What It Does

The AI Checker analyzes TRLC requirements and architectural design artefacts
against engineering guidelines using an AI model.  For each artefact it
produces:

- a list of **findings** (categorized as *Major* or *Minor*)
- a list of **suggestions** for improvement
- a numerical **quality score** from 0 to 10

Results are written as a JSON envelope plus HTML and reStructuredText reports.

### Prerequisites

- A GitHub Copilot license (default backend) **or** a custom AI model
  (see [Custom AI Model](#custom-ai-model))
- Bazel

### Running a Check

Add a rule to your `BUILD` file and run it like any other test — the rule bakes
the required environment-variable inheritance (credentials + proxy) into the
target, so no extra `--config` or `--test_env` flag is needed:

```starlark
load("@score_tooling//validation/ai_checker:ai_checker.bzl", "trlc_requirements_ai_test")

trlc_requirements_ai_test(
    name = "requirements_ai_check",
    reqs = [":my_requirements"],
    score_threshold = "6.0",
    tags = ["manual"],
)
```

```bash
bazel test //path/to:requirements_ai_check
```

The `tags = ["manual"]` attribute is recommended to prevent the rule from
running during routine `bazel test //...` sweeps.

### Rule Reference

#### `trlc_requirements_ai_test`

Analyzes TRLC requirements against the built-in requirements engineering
guidelines.

```starlark
trlc_requirements_ai_test(
    name = "requirements_ai_check",
    reqs = [":my_requirements"],           # required: targets providing TrlcProviderInfo
    model = "claude-sonnet-4.6", # optional: AI model to use
    score_threshold = "6.0",              # optional: minimum average score to pass (0–10)
    guidelines = "//my/org:guidelines",   # optional: override default guideline filegroup
    tags = ["manual"],
)
```

| Attribute | Description | Required | Default |
|-----------|-------------|----------|---------|
| `reqs` | Label list of targets providing `TrlcProviderInfo` | Yes | — |
| `model` | AI model identifier | No | `"claude-sonnet-4.6"` |
| `score_threshold` | Minimum average score (0–10) to pass the test | No | `"0.0"` |
| `guidelines` | Filegroup of guideline markdown files | No | `default_guidelines` |
| `context` | Filegroup of background-context files (`.md` / `.puml`) injected as read-only reference material | No | — |

#### `architecture_ai_test`

Analyzes architectural design artefacts against the built-in architecture
guidelines.

```starlark
architecture_ai_test(
    name = "architecture_ai_check",
    designs = [":my_architectural_design"],  # required: targets providing ArchitecturalDesignInfo
    model = "claude-sonnet-4.6",
    score_threshold = "6.0",
    tags = ["manual"],
)
```

| Attribute | Description | Required | Default |
|-----------|-------------|----------|---------|
| `designs` | Label list of targets providing `ArchitecturalDesignInfo` | Yes | — |
| `model` | AI model identifier | No | `"claude-sonnet-4.6"` |
| `score_threshold` | Minimum average score (0–10) to pass the test | No | `"0.0"` |
| `guidelines` | Filegroup of guideline markdown files | No | `default_architecture_guidelines` |
| `context` | Filegroup of background-context files (`.md` / `.puml`) injected as read-only reference material | No | — |

> Architecture review reads the **raw PlantUML source** of the design's
> diagrams (not the parsed FlatBuffers binaries).

### Output

The AI analysis runs **at test time** (the test action launches the analysis),
so the reports are written to the test's undeclared-outputs directory and packed
into the test log archive. Each test produces three report files (one set per
test, so the names are fixed rather than prefixed):

| File | Content |
|------|---------|
| `analysis.json` | Self-contained report envelope: `metadata`, `guidelines`, `analyses` (scores, findings, suggestions) |
| `analysis.html` | Interactive HTML report |
| `analysis.rst` | Standalone reStructuredText report |

The HTML report shows a color-coded score card per artefact, linked guideline
reference pages, and summary statistics. The JSON is a self-contained envelope
(model/timestamp/git metadata + guideline texts + per-artefact analyses), so it
fully captures the report.

Retrieve the reports after a test run from the undeclared-outputs archive:

```bash
bazel test //path/to:requirements_ai_check
unzip -o bazel-testlogs/path/to/requirements_ai_check/test.outputs/outputs.zip -d /tmp/ai_report
```

### Debug Output

A verbose debug log (`debug.log`) is always written alongside the reports
in the same undeclared-outputs archive. It contains the raw prompt sent to the
AI model and response timing. Extract it the same way:

```bash
bazel test //path/to:requirements_ai_check
unzip -p bazel-testlogs/path/to/requirements_ai_check/test.outputs/outputs.zip \
  debug.log
```

(custom-ai-model)=
### Custom AI Model

To use a provider other than the default Copilot SDK agent, point
`_custom_ai_model` at a `py_binary` or `py_library` target that exposes a
`create_agent()` function returning an `AnalysisAgent`:

```starlark
trlc_requirements_ai_test(
    name = "requirements_ai_check",
    reqs = [":my_requirements"],
    _custom_ai_model = "//my/org:ai_model_py",
)
```

See the [Integration Guide](#integration-guide) for full details on implementing
a custom agent.

---

(integration-guide)=
## Integration Guide

This section describes how to use the AI Checker from another Bazel repository
(e.g., a consumer workspace that references this repo via a Bazel registry or
`git_repository`).

### Step 1 — Provide Credentials

The AI analysis runs at **test time**, and the test rules bake the required
environment-variable inheritance into each target via `RunEnvironmentInfo`. When
you run `bazel test`, the test inherits `HOME`, the GitHub tokens, and the proxy
variables from your shell automatically — there is **no** `--config=copilot` or
`--test_env` flag to set, and nothing to copy into your root `.bazelrc`.

> `HOME` matters because the test runner otherwise resets it to a private temp
> directory, which would hide the Copilot CLI's `~/.copilot/config.json`.

Just make sure one credential source is present in your shell before running the
test (see the table below).

Optionally, import the bundled `.bazelrc.ai_checker` to enable the
project-specific guideline flags (it contains **no** environment configuration):

```text
try-import %workspace%/.bazelrc.ai_checker
```

**Authentication** — at least one of the following must be available in your
environment:

| Variable | Purpose |
|----------|---------|
| `COPILOT_GITHUB_TOKEN` | Explicit token — recommended for CI |
| `GH_TOKEN` | GitHub CLI compatible |
| `GITHUB_TOKEN` | GitHub Actions compatible |
| `HOME` | Lets the CLI find stored OAuth credentials in `~/.copilot/` |

### Step 2 — Declare Bazel Targets

```starlark
load("@score_tooling//validation/ai_checker:ai_checker.bzl",
     "trlc_requirements_ai_test",
     "architecture_ai_test")

# Analyze TRLC requirements
trlc_requirements_ai_test(
    name = "requirements_ai_check",
    reqs = [":my_requirements"],           # target providing TrlcProviderInfo
    model = "claude-sonnet-4.6",
    score_threshold = "6.0",              # fail if average score < 6.0
    tags = ["manual"],                    # recommended: exclude from //...
)

# Analyze architectural designs
architecture_ai_test(
    name = "architecture_ai_check",
    designs = [":my_architectural_design"],  # target providing ArchitecturalDesignInfo
    model = "claude-sonnet-4.6",
    score_threshold = "6.0",
    tags = ["manual"],
)
```

**Manual tag recommendation:** Adding `tags = ["manual"]` prevents accidental
AI analysis runs during routine `bazel test //...` sweeps.  Run AI tests
by targeting them explicitly:

```bash
bazel test //path/to:requirements_ai_check
```

| Attribute | Description | Required | Default |
|-----------|-------------|----------|---------|
| `reqs` / `designs` | Targets providing `TrlcProviderInfo` or `ArchitecturalDesignInfo` | Yes | — |
| `model` | AI model identifier | No | `"claude-sonnet-4.6"` |
| `score_threshold` | Minimum average score (0–10) to pass | No | `"0.0"` |
| `guidelines` | Custom guideline filegroup | No | `default_guidelines` / `default_architecture_guidelines` |
| `context` | Background-context filegroup (`.md` / `.puml`), read-only reference material | No | — |

### Guidelines

Guidelines are layered, so projects only supply what is specific to them:

| Layer | Scope | Source |
|-------|-------|--------|
| **General** | Review methodology, scoring, result format — applies to every element type | `guidelines/general.md` |
| **Type** | Generic rules for one element type (requirements *or* architecture) | `guidelines/requirements/` · `guidelines/architecture/` |
| **Project** | Project-specific details (e.g. requirement levels, architecture levels) | Set via a flag — see below |

The general and type layers are built in. To override them per target, set the
`guidelines` attribute to your own filegroup:

```starlark
trlc_requirements_ai_test(
    name = "my_ai_check",
    reqs = [":my_requirements"],
    guidelines = "//my/org:custom_guidelines",
)
```

### Project-Specific Guidelines (set once)

Project details are injected as **graded** rules via label flags, so you set
them once in `.bazelrc` instead of on every target:

```text
build --//validation/ai_checker:project_guidelines=//my/org:my_req_guidelines
build --//validation/ai_checker:project_architecture_guidelines=//my/org:my_arch_guidelines
```

Each flag points at a `filegroup` of `.md` files. Bundled SCORE examples are
available as `//validation/ai_checker:score_project_guidelines` and
`//validation/ai_checker:score_project_architecture_guidelines`. When unset, no
project guidelines are added.


### Custom AI Model (Bazel)

To substitute a different AI backend at the Bazel level, provide a
`_custom_ai_model` attribute pointing to your `ai_model.py` file:

```starlark
trlc_requirements_ai_test(
    name = "requirements_ai_check",
    reqs = [":my_requirements"],
    _custom_ai_model = "//my/org:ai_model_py",
)
```

The file must expose `create_agent(model_name) -> AnalysisAgent`. The agent
implements a single async method:

```python
async def analyze(self, system_prompt: str, artefacts_text: str) -> AnalysisResults
```

To reuse a LangChain model, return the bundled `LangChainAgent` wrapper:

```python
from ai_checker.agents.langchain_agent import LangChainAgent

def create_agent(model_name):
    return LangChainAgent(MyLangChainChatModel(model=model_name))
```

### Debug Output

To inspect the raw input sent to the AI model and response timing, extract the
always-on debug log from the test's undeclared-outputs archive:

```bash
bazel test //path/to:requirements_ai_check
unzip -p bazel-testlogs/path/to/requirements_ai_check/test.outputs/outputs.zip \
  debug.log
```

The debug log contains:
- Python version, model name, and guidelines path
- Batch processing information
- Complete system message (guidelines) and human message (artefacts)
- Response timing and token cost statistics

---

## Developer Guide

Architecture, agent internals, the report pipeline, caching, and extension
points are documented in [DEVELOPMENT.md](https://github.com/eclipse-score/tooling/blob/main/validation/ai_checker/DEVELOPMENT.md).
