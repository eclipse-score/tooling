<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# libclang Integration Tests

This directory contains integration tests for the C++ libclang parser and related tooling.

## Directory Structure

- `cases/`: Each subdirectory is an independent test case, containing C++ sources, BUILD files, and golden `expected.json` outputs.
- `test_framework.rs`: Rust test framework that invokes the parser and compares the debug JSON sidecar to the golden file.
- `BUILD`: Bazel build and test rules for integration.

## Test Workflow

1. Each case directory contains C++ source files, headers, a BUILD file, and an `expected.json` golden output.
2. The case calls the shared integration test macro:
   ```starlark
   cpp_parser_integration_test(
       name = "test_case_library",
       target = ":case_library",
       expected_output = ["expected.json"],
   )
   ```
   The macro creates the expected output filegroup and parser target, exposes `CppParserInfo.debug_json` through the test-only debug JSON target, and wires the Rust comparison test.
3. The Rust test framework receives that debug JSON path and compares it to `expected.json`.
4. To batch test all cases:

```bash
bazel test --test_output=all --nocache_test_results //cpp/libclang/integration_test/...
```

To add a new case, follow the structure and BUILD conventions of the existing `cases` subdirectories.
