# libclang Integration Tests

This directory contains integration tests for the C++ libclang parser and related tooling.

## Directory Structure

- `cases/`: Each subdirectory is an independent test case, containing C++ sources, BUILD files, and golden `expected.json` outputs.
- `test_framework.rs`: Rust test framework that invokes the parser and compares output to the golden file.
- `BUILD`: Bazel build and test rules for integration.

## Test Workflow

1. Each case directory contains C++ source files, headers, a BUILD file, and an `expected.json` golden output.
3. The Rust test framework uses the parser to process the case and compares the output to `expected.json`.
4. To batch test all cases:

```bash
bazel test --test_output=all --nocache_test_results //cpp/libclang/integration_test/...
```

To add a new case, follow the structure and BUILD conventions of the existing `cases` subdirectories.
