<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Coverage

## Combined Rust + Python Coverage

The `//coverage:combined_report` target generates a single HTML coverage report
for all Rust and Python tools in the repository using Bazel's built-in
coverage support (`bazel coverage`) and `genhtml`.

### Usage

```bash
bazel run //coverage:combined_report
```

This runs `bazel coverage --config=coverage` for `//plantuml/...`,
`//validation/...` and `//manual_analysis/...`, merges all LCOV data, and
renders the report to `<workspace>/coverage-html/index.html`.

Custom output directory:

```bash
bazel run //coverage:combined_report -- --out-dir /tmp/my-coverage
```

Custom target set:

```bash
bazel run //coverage:combined_report -- --targets "//plantuml/... //validation/core/..."
```

### How it works

1. `bazel coverage --config=coverage` compiles Rust with `-Cinstrument-coverage`
   and wraps Python tests with `coverage.py` (via `rules_python`'s built-in
   `configure_coverage_tool`).
2. Bazel merges all per-test LCOV files into one `_coverage_report.dat`
   (controlled by `--combined_report=lcov`).
3. `--instrumentation_filter` limits instrumentation to the three tool
   packages, excluding external dependencies and generated code.
4. Test infrastructure files (`integration_test/`, `tests/`) are excluded from
   instrumentation via `--instrumentation_filter`; external Python files are
   removed via `lcov --remove`.
5. The HTML report uses a high-coverage threshold of **95 %** (green) and the
   default medium threshold of 75 % (yellow).
6. `genhtml` and `lcov` are downloaded hermetically via the `download_utils`
   Bazel module (`@lcov_deb`) — no system installation of `lcov` is required.

### .bazelrc config

The `coverage:coverage` config in `.bazelrc` provides the required flags:

```
coverage:coverage --combined_report=lcov
coverage:coverage --instrumentation_filter=//plantuml,//validation,//manual_analysis,-//plantuml/parser/integration_test,-//validation/core/integration_test
coverage:coverage --@rules_rust//rust/settings:extra_rustc_flag=-Clink-dead-code
coverage:coverage --@rules_rust//rust/settings:extra_rustc_flag=-Ccodegen-units=1
```

You can also run `bazel coverage` directly without the script (requires `genhtml`
from the system `lcov` package):

```bash
bazel coverage --config=coverage //plantuml/... //validation/... //manual_analysis/...
genhtml "$(bazel info output_path)/_coverage/_coverage_report.dat" \
  --output-directory coverage-html/
```
