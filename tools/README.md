# S-CORE linter

This bazel module provides centrally managed binaries for linter and static analysis tools used in S-CORE.

It provides a template script `sample.sh` that can be adapted to run the tools you need.

## Current tools:
Currently binaries / executables for the following tools are provided
-  **Ruff**: A super-fast Python linter.
-  **basedpyright**: A type checker for Python.
-  **Actionlint**: A linter for your GitHub Actions workflows.
-  **Shellcheck**: A static analysis tool for shell scripts.
-  **Yamlfmt**: A handy formatter for YAML files.

## How to use the Module
Add the import of `multitool` as well as `score_linter` to your `MODULE.bazel` file.
Adapt the `use_repo` and `register_toolchains` calls to only import/use the tools you need.
```
bazel_dep(name = "score_tooling", version = "1.0.0")
bazel_dep(name = "rules_multitool", version = "1.8.0")

multitool_root = use_extension("@rules_multitool//multitool:extension.bzl", "multitool")
use_repo(multitool_root, "actionlint_hub", "multitool", "ruff_hub", "shellcheck_hub", "yamlfmt_hub")

register_toolchains(
    "@ruff_hub//toolchains:all",
    "@actionlint_hub//toolchains:all",
    "@shellcheck_hub//toolchains:all",
    "@yamlfmt_hub//toolchains:all",
)
```

### Run the Lint Script (sample.sh)

Copy the [sample.sh script](https://github.com/eclipse-score/tooling/tools/sample.sh).

Adapt it to only run the tools you need, by deleting or commenting out the lines not necessary. The script will run all the configured linters and report any issues it finds.

Ensure the script is executable `chmod u+x <script name>`.

You now can simply run it via `./<script name>` and should see all the output for your project.