<!--
Copyright (c) 2026 Contributors to the Eclipse Foundation

SPDX-License-Identifier: Apache-2.0
-->

# Manual Analysis

This directory provides Bazel rules and tooling for defining and executing
**manual verification analyses**. A manual analysis captures a structured,
interactive review of a piece of implementation (the *context*) against a set
of requirements. A lock file ensures the analysis is re-done whenever the
context changes.

## Overview

A manual analysis consists of:

- **Context** – the implementation under review (source files, headers, or
  entire `cc_library` dependency graphs).
- **Analysis YAML** – a structured, step-by-step description of the
  verification procedure (`action`, `automated_action`, `decision`,
  `assertion`, `repeat`).
- **Results file** – a JSON file capturing the outcomes recorded during an
  interactive run.
- **Lock file** – a SHA-256 digest file that ties the results to a specific
  snapshot of the context. If the context changes, the test fails and the
  analysis must be re-executed.

The tooling also produces a **LOBSTER traceability artifact** (`.lobster` file)
so that coverage of requirements can be tracked across the project.

## Bazel Rules

### `manual_analysis` (macro)

The primary entry point. It instantiates two targets:

| Target | Kind | Purpose |
|---|---|---|
| `{name}` | `test` | Checks that the lock file is current and generates the `.lobster` output |
| `{name}.update` | `run` | Runs the interactive analysis and refreshes the lock file |

**Attributes**

| Attribute | Required | Description |
|---|---|---|
| `contexts` | ✓ | List of context-provider targets (`ManualAnalysisContextInfo`) |
| `analysis` | ✓ | Label of the analysis YAML file |
| `lock_file` | ✓ | Label of the lock file (workspace-relative, committed to VCS) |
| `results_file` | ✓ | Label of the results JSON file (workspace-relative, committed to VCS) |

The `contexts` attribute accepts **any** target that provides
`ManualAnalysisContextInfo`. The two rules below are convenience examples.
You can define additional project-specific context-provider rules as needed.

### `manual_analysis_context_from_filegroup`

The rule wraps an arbitrary `filegroup` (or any target with `DefaultInfo`)
and exposes its files as `ManualAnalysisContextInfo`.

```starlark
load("//manual_analysis:context_from_filegroup.bzl", "manual_analysis_context_from_filegroup")

manual_analysis_context_from_filegroup(
    name = "my_context",
    filegroup = ":my_sources",
)
```

### `manual_analysis_context_from_cc_library`

The rule traverses a `cc_library` and all its transitive dependencies. It
collects source and header files as well as selected build attributes
(`copts`, `defines`, etc.) so that any change to the compilation units is
reflected in the lock file. The compiled output is explicitly not captured
to avoid lock file invalidation when the toolchain is modified. 

```starlark
load("//manual_analysis:context_from_cc_library.bzl", "manual_analysis_context_from_cc_library")

manual_analysis_context_from_cc_library(
    name = "my_cc_context",
    library = ":my_library",
)
```

### Custom context-provider rules

If your project needs a different notion of context, define a custom rule that
returns `ManualAnalysisContextInfo`.

```starlark
load("//manual_analysis:manual_analysis.bzl", "ManualAnalysisContextInfo")

def _my_context_impl(ctx):
    files = depset(ctx.files.srcs)
    return [
        DefaultInfo(files = files),
        ManualAnalysisContextInfo(
            files = files,
            # Optional: serialized "<label>\t<canonical-attributes>" entries
            # that should influence lock-file computation.
            rules = depset(),
        ),
    ]

my_manual_analysis_context = rule(
    implementation = _my_context_impl,
    attrs = {
        "srcs": attr.label_list(allow_files = True),
    },
)
```

Use your custom target in `manual_analysis(contexts = [...])` exactly like the
built-in helper rules.

## Analysis YAML Schema

The analysis YAML describes the verification procedure. Supported step types:

| Step type | Key fields | Description |
|---|---|---|
| `action` | `description` | A manual action the reviewer performs |
| `automated_action` | `command`, `args`, `expected_return_code` | A shell command run by the tool; result is captured automatically |
| `decision` | `description`, `branches` | A branching point; the reviewer picks one branch |
| `assertion` | `description`, `positive`, `negative` | A yes/no question; `positive` is the passing answer |
| `repeat` | `until`, `steps` | A loop that repeats its steps until the `break` answer is chosen |

**Example:**

```yaml
requirements:
  - "my_component.REQ_001"

steps:
  - action:
    description: Identify all public API entry points
  - automated_action:
    description: List all callers of the unsafe function
    command: "{bazel} cquery \"rdeps({root}, {target})\" --notool_deps"
    args:
      - name: bazel
        default: bazel
      - name: root
      - name: target
  - decision:
    description: Does any caller pass unvalidated input?
    branches:
      - answer: Yes
        steps:
          - assertion:
            description: Is input validation present before the unsafe call?
            positive: Yes
            negative: No
      - answer: No
        steps: []
  - assertion:
    description: Is the component safe with respect to REQ_001?
    positive: Yes
    negative: No
```

## Typical Workflow

### 1. Define context and analysis in BUILD

```starlark
load("//manual_analysis:context_from_cc_library.bzl", "manual_analysis_context_from_cc_library")
load("//manual_analysis:manual_analysis.bzl", "manual_analysis")

# You can use a built-in helper rule or a custom context-provider rule.
manual_analysis_context_from_cc_library(
    name = "my_context",
    library = ":my_library",
)

manual_analysis(
    name = "my_analysis",
    contexts = [":my_context"],
    analysis = "analysis.yaml",
    lock_file = "my_analysis.lock",
    results_file = "results.json",
)
```

### 2. Perform the interactive analysis

Run the update target to execute the analysis interactively. The tool guides
you through each step, records your answers in `results.json`, and updates the
lock file:

```bash
bazel run //my_package:my_analysis.update
```

### Interactive UI

The interactive runner uses a full-screen split-pane terminal UI.

- **Left pane (`Analysis Progress`)** shows the running history of all steps,
  answers, and command output.
- **Right pane(s)** are used for current input (instructions, text fields,
  answer selection, or argument forms depending on step type).
- **Keyboard shortcuts**:
  - `Tab` / `Shift-Tab` switches focus between panes/fields.
  - `Ctrl-S` or `F2` submits the current prompt.
  - `Ctrl-C` aborts the run.
  - `F4` opens `$VISUAL` / `$EDITOR` for multiline text prompts.

Step behavior in the UI:

- `action`: enter free-form notes in a multiline field.
- `decision` / `assertion`: choose one allowed answer and optionally add a
  justification.
- `automated_action`: fill argument values, then the resolved command is
  executed and its output is streamed into `Analysis Progress`.
- `repeat`: execute nested steps, then answer the repeat-until prompt
  (`continue` vs `break`).

By default, previous `results.json` content is used to prefill prior answers
and argument values where possible. Disable this with:

```bash
bazel run //my_package:my_analysis.update -- --no-prefill-from-last-run
```

### 3. Commit the generated files

Commit both the updated lock file and the results file to version control so
that the test target can verify them in CI:

```bash
git add my_analysis.lock results.json
git commit -m "feat: perform manual analysis for my_component"
```

### 4. Verify in CI

The test target checks that the lock file matches the current context and that
all assertions in the results file passed:

```bash
bazel test //my_package:my_analysis
```

If the context changes (source files are modified, build attributes are
updated, etc.) the test will fail with a lock mismatch, prompting the
reviewer to repeat the analysis.

## LOBSTER Integration

The test target produces a `.lobster` file that can be consumed by
[LOBSTER](https://github.com/bmw-software-engineering/lobster) to link the
analysis to upstream requirements. Pass the output to a `lobster_test` or
`lobster_report` target:

```starlark
load("@lobster//:lobster.bzl", "lobster_test")

lobster_test(
    name = "traceability_test",
    activities = ["//my_package:my_analysis"],
    ...
)
```

## Example

A fully working example is available in [`example/`](example/). It
demonstrates:

- Context from a `filegroup` (`context_a.txt`, `context_b.txt`)
- Context from a `cc_library` (`ma_cc_root` → `ma_cc_dep`)
- A complete analysis YAML with `action`, `automated_action`, `decision`,
  `repeat`, and `assertion` steps
- Integration with `lobster_trlc` and TRLC requirements

