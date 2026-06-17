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
2. The case `cpp_parser(...)` target must set `emit_debug_json = True` so the parser emits the aggregated `debug.json` sidecar required by the test harness.
3. The Rust test framework reads `debug.json` from the parser output directory and compares it to `expected.json`.
4. To batch test all cases:

```bash
bazel test --test_output=all --nocache_test_results //cpp/libclang/integration_test/...
```

To add a new case, follow the structure and BUILD conventions of the existing `cases` subdirectories.
