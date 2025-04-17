# StarPLS Bazel Module

This Bazel module provides a convenient way to integrate the pre-built `starpls` Starlark language server into your Bazel workspace.

## Features

*   Downloads a specific version of the `starpls` binary.
*   Provides a simple Bazel macro (`setup_starpls`) to create a `genrule` target that downloads and makes the language server executable.

## Prerequisites

*   An operating system compatible with the downloaded binary (currently hardcoded to `linux-amd64`).
*   The `rules_shell` Bazel module must be added as a dependency in your workspace's `MODULE.bazel` file.

## Usage

1.  **Add dependencies to your `MODULE.bazel` file:**

    ```bazel
    # Needed by the score_starpls_lsp module internally
    bazel_dep(name = "rules_shell", version = "0.4.0") # Or newer

    # The score_starpls_lsp module itself
    bazel_dep(name = "score_starpls_lsp", version = "0.1.0")
    # If using a local path override for testing:
    # local_path_override(
    #     module_name = "score_starpls_lsp",
    #     path = "/path/to/your/local/score_starpls_lsp" 
    # )
    ```

2.  **Load the setup macro and invoke it in your `BUILD` file:**

    ```bazel
    load("@score_starpls_lsp//:starpls.bzl", "setup_starpls")

    setup_starpls(
        name = "starpls_server", 
        visibility = ["//visibility:public"],
    )
    ```

3.  **Run the language server:**

    You can now run the language server directly using Bazel:
    ```bash
    bazel run //starpls_server-- [arguments_for_starpls...] 
    ```

4.  **Configure your IDE (e.g., VS Code):**

    Point your IDE's Starlark/Bazel language server setting to the executable `genrule` target you created (e.g., `//starpls_server`). For VSCode, configure it to execute the `bazel run` command:
    ```json
        "bazel.lsp.command": "bazel",
        "bazel.lsp.args": [
            "run",
            "//:starpls_server"
        ],
    ```

## Current Limitations

*   **Platform:** Downloads only the `linux-amd64` binary.
*   **Version:** Downloads a fixed version (`0.1.21`) of `starpls`.

## Running Integration Tests

To run the `starpls_test` integration test suite after cleaning the Bazel cache and outputs, execute the following command from the root directory of the `starpls` module:

```bash
bazel clean --expunge && bazel test //integration_tests:starpls_test