<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Run C++ parser targets

## Configure a parser target in `BUILD`

If you want to parse a specific Bazel target, use the `cpp_parser(...)` rule in the `BUILD` file like:

```
load("//cpp/libclang:cpp_parser.bzl", "cpp_parser")

cpp_parser(
  name = "cpp_parser_include_3rdparty",
  emit_debug_json = True,
  extra_args = [
  ],
  target = "//cpp/libclang/integration_test/cases/include_3rdparty",
)
```

Where:

- `target` is the Bazel target you want to parse.
- `emit_debug_json` is optional and defaults to `False`. Enable it when you want the aggregated `debug.json` sidecar.

Expected result:

- Bazel creates parser output artifact:
  - `bazel-bin/cpp/libclang/cpp_parser_include_3rdparty_class_diagram.fbs.bin`
- When `emit_debug_json = True`, the parser also writes:
  - `bazel-bin/cpp/libclang/cpp_parser_include_3rdparty_debug.json`

## Configure debug logging

To enable debug output for parser actions, set the Bazel build setting:

```bash
bazel build //cpp/libclang/integration_test/cases/include_3rdparty:parser --//cpp/libclang:log_level=debug
```

Accepted values are: `error`, `warn`, `info`, `debug`, `trace`.

## Quick check (optional)

```bash
ls -l bazel-bin/cpp/libclang/integration_test/cases/include_3rdparty/parser_class_diagram.fbs.bin
ls -l bazel-bin/cpp/libclang/integration_test/cases/include_3rdparty/parser_debug.json
```
