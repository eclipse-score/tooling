<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Agent Guidance

`score_tooling` is a published Bazel module bundling independent tools: Rust (`plantuml`,
`validation`, `tools`, `cpp/libclang`), Python, C++ and Starlark rules. Each package has its own
README, conventions and tests; nested `AGENTS.md` files refine this one.

## Every Task

- Read the package README, BUILD and neighbouring tests before editing; the
  [tool index](README.md#available-tools) lists the docs.
- Keep changes scoped; reuse local patterns; leave unrelated working-tree changes alone.
- Public rules, macros, attrs and `defs.bzl` exports are consumed downstream: keep them compatible.
  Deprecate with a visible warning naming the replacement, document the removal release, test the
  old spelling; remove only in a later major release.
- Behavior changes need tests. Run the narrowest target (`bazel test //pkg/...`), not `//...`.
- Comments: if required, one short line stating current behavior or a non-obvious why. No history, no
  change narration; trim long comments you touch.
- Docs: short and concise, current state only; no long explanations or changelog prose.
- Report what ran and what is unverified.

## Scopes

| Directory | Contents |
|-----------|----------|
| [bazel/rules/rules_score](bazel/rules/rules_score/AGENTS.md) | SEooC rules, Sphinx docs assembly (own `AGENTS.md`) |
| `bazel/rules/{exec_in_sysroot,sysroot_from_lock}` | Sysroot helper rules |
| `plantuml/` | Rust PlantUML parser (`parser/`), Sphinx extension (`sphinx/`) |
| `validation/` | Rust diagram validation (`core/`), `ai_checker/` |
| `cpp/libclang/` | C++ to class-diagram parser (Rust + libclang) |
| `manual_analysis/`, `lobster_bazel/` | Manual analysis rules, source tracing |
| `coverage/` | LLVM C++/Rust coverage pipeline |
| `cr_checker/`, `dash/`, `cli_helper/`, `starpls/`, `python_basics/`, `skills_sync/` | Single-purpose Bazel tools, each with README |
| `tools/`, `third_party/{format,lint}` | Formatter/linter setup, LOBSTER RST report, metamodel |
| `third_party/{docs_runtime,tooling_sysroot,plantuml}` | Vendored runtimes and sysroot |
| `docs/` | Repository documentation |

## Repo Specifics

- Directories with their own `MODULE.bazel` (`bazel/rules/rules_score/test`,
  `bazel/rules/rules_score/examples/*`, `python_basics/integration_tests`,
  `starpls/integration_tests`, `cr_checker/tests`) are separate module roots, not covered by
  root `bazel test`. `cd` into them and run `bazel test //...`; CI does this (`.github/workflows/tests.yml`).
- Rust crates are also checked with `bazel build //<pkg>/... --config=clippy`.
- Python lint: `--config=lint` (ruff, ty); line length 120 (`pyproject.toml`).
- After changing `MODULE.bazel`, run `bazel mod deps` and commit `MODULE.bazel.lock`;
  CI requires `bazel mod deps --lockfile_mode=error` to pass.
- Before committing: `bazel run //:format.fix` (buildifier, formatters) and
  `bazel run //:copyright-fix` (license headers on new files).
- `.github/skills/score-*` are user-facing docs shipped to downstream repos via `//:sync_skills`.
  They are not developer guidance; update them only when user-visible behavior changes.
