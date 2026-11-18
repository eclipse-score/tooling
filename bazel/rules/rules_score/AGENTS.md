<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# rules_score Agent Guidance

Developer notes for the `rules_score` Bazel rules; they refine the [root AGENTS.md](../../../AGENTS.md).
Architecture map: [docs/tooling_architecture.rst](docs/tooling_architecture.rst).

## Layout

- `private/*.bzl`: rule implementations. `providers.bzl`: provider contracts between rules.
  `rules_score.bzl`: the only public entry point; every public rule or provider is re-exported there.
- `src/`: Python helpers run by actions (tests live in `test/test_*.py`).
  `templates/`: Sphinx/RST templates. `lobster/config/`: LOBSTER configs. `trlc/config/`: metamodel
  (`.rsl`) and tool qualification records (`tools.trlc`).
- `README.md` (per-rule reference), `docs/` (user guide, architecture, tool qualification).

## Changing a Rule

- New or changed rule/attr: implement in `private/`, re-export in `rules_score.bzl`, add the
  provider to `providers.bzl` if data flows to other rules, and update the `README.md` section.
- Tests: add to `test/` (analysis tests in `*_test.bzl`, registered in `test/BUILD`).
  Reuse the helpers in `test/lib` and fixtures in `test/fixtures`.
- User-visible change: also update `docs/`, the matching example under `examples/`, and the
  `score-*` skill in `.github/skills`.
- New or changed tool: update `docs/tooling_architecture.rst`, `trlc/config/tools.trlc` and the
  qualification records in `docs/requirements/`.
- Python deps: edit `requirements.in`, then regenerate (see the comment in `BUILD`).

## Verify

`test/` and `examples/*` are separate modules and use `score_tooling` via `local_path_override`:

```bash
cd bazel/rules/rules_score/test && bazel test //...
cd bazel/rules/rules_score/examples/<name> && bazel build //... && bazel test //...
bazel test -- //bazel/rules/rules_score/... -//bazel/rules/rules_score/examples/...   # from repo root
```

Run the narrow target first, and the examples when a rule's behavior or attrs change.
Add `--config=info` or `--config=debug` (root `.bazelrc`) for verbose rule output.
