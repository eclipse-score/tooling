<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

-  **Ruff**: A super-fast Python linter.
-  **Actionlint**: A linter for your GitHub Actions workflows.
-  **Yamlfmt**: A handy formatter for YAML files.

Ruff and Yamlfmt are provided directly by `aspect_rules_lint`'s own
bundled multitool hub (the default `@multitool` hub) -- consumers should add
their own `bazel_dep(name = "aspect_rules_lint", ...)` and reference
`@aspect_rules_lint//lint:ruff_bin`, `@aspect_rules_lint//format:yamlfmt`,
etc. `score_tooling` doesn't re-expose
them: `aspect_rules_lint` is a `dev_dependency` of `score_tooling`, so its
`@multitool` hub entries aren't visible to repos that merely depend on
`score_tooling`. `score_tooling` only maintains its own dedicated hub for
Actionlint, which `aspect_rules_lint` doesn't bundle, and exposes it as
`@score_tooling//tools:actionlint`.

## How to use the Module
Running Actionlint via `bazel run @score_tooling//tools:actionlint` (see
`tools/BUILD`) works out of the box for any repo that depends on
`score_tooling` -- no extra `MODULE.bazel` setup is required.

If you want direct access to the underlying hub in your own `MODULE.bazel`
(e.g. to register the toolchain yourself), add:
```
bazel_dep(name = "score_tooling", version = "1.0.0")
bazel_dep(name = "rules_multitool", version = "1.9.0")

multitool_root = use_extension("@rules_multitool//multitool:extension.bzl", "multitool")
use_repo(multitool_root, "actionlint_hub")

register_toolchains(
    "@actionlint_hub//toolchains:all",
)
```

For Ruff and Yamlfmt, depend on `aspect_rules_lint` directly:
```
bazel_dep(name = "aspect_rules_lint", version = "2.5.0")
```


### Run the Lint Script (sample.sh)

Copy the [sample.sh script](https://github.com/eclipse-score/tooling/blob/main/tools/sample.sh).

Adapt it to only run the tools you need, by deleting or commenting out the lines not necessary. The script will run all the configured linters and report any issues it finds.

Ensure the script is executable `chmod u+x <script name>`.

You now can simply run it via `./<script name>` and should see all the output for your project.
