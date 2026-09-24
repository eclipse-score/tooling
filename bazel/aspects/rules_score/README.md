<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# `dependable_element_export` aspect

Collects the doc/report artifacts of every `dependable_element()` (rules_score SEooC) reached by
a build and republishes them under a uniform `_dependable_element_export/` path segment, as a
dedicated, explicitly requestable output group.

It is designed to be added to an **existing** build invocation without changing what that build
produces for any other target: the aspect no-ops for every target that does not carry the
`DependableElementInfo` provider, and its outputs are only built when you ask for the
`dependable_element_export` output group.

## Usage

```sh
bazel build //<your_target(s)> \
    --aspects=@score_tooling//bazel/aspects/rules_score:dependable_element_export.bzl%dependable_element_export_aspect \
    --output_groups=+dependable_element_export \
    --remote_download_regex=".*/_dependable_element_export/.*"
```

The three flags are independent and all required:

| Flag | Why |
| --- | --- |
| `--aspects=` | Attaches the aspect. Note the value is `<label of the .bzl file>%<aspect symbol>` — **not** a target label. |
| `--output_groups=+dependable_element_export` | The leading `+` *adds* to the default output groups instead of replacing them, so the original build still produces everything it normally would. |
| `--remote_download_regex=` | Only needed with remote execution (`--config=rbe`). Without it the exported artifacts stay on the remote worker and never materialize locally. |

Safe to apply to a wide pattern such as `//...` — packages without a `dependable_element()` cost
nothing.

## Where the output lands

```text
bazel-bin/<package-path>/_dependable_element_export/
```

To find every exported element across a whole workspace after the build:

```sh
find -L bazel-bin -type d -name _dependable_element_export
```

## Why the artifacts are copied rather than just re-exposed

A naive aspect would simply re-advertise each `dependable_element`'s existing default outputs.
That is not enough for the CI use case, because those files live at unpredictable,
rule-internal paths that differ per element, which makes them hard to glob, hard to filter with
`--remote_download_regex`, and hard to upload under a stable directory structure.

So the aspect declares new outputs under `_EXPORT_DIR` and populates them with explicit copy
actions. Two details matter:

- **Copies, not symlinks.** `ctx.actions.symlink()` was tried first and broke under remote
  execution — the symlinks were not materialized usefully on the local side.
- **Tree artifacts are handled separately.** Some `dependable_element` outputs are directories
  (e.g. generated Sphinx HTML), so the implementation branches on `File.is_directory` and uses
  `declare_directory()` + a recursive copy for those.

The package prefix is stripped from each file's `short_path` before relocation, so paths are not
redundantly nested inside themselves.

