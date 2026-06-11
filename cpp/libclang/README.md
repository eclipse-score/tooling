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
  extra_args = [
  ],
  target = "//cpp/libclang/integration_test/cases/include_3rdparty",
  tool = ":clang_rs_parser",
)
```

Where:

- `target` is the Bazel target you want to parse.

Expected result:

- Bazel creates parser output artifact:
  - `bazel-bin/cpp/libclang/cpp_parser_include_3rdparty_result.json`

## Quick check (optional)

```bash
ls -l bazel-bin/cpp/libclang/cpp_parser_include_3rdparty_result.json
