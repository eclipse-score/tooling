<!--
Copyright (c) 2026 Contributors to the Eclipse Foundation

SPDX-License-Identifier: Apache-2.0
-->

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

---

## Ferrocene Rust Coverage

This directory provides the Ferrocene Rust coverage workflow for Bazel-based
projects. It uses Ferrocene's `symbol-report` and `blanket` tools to generate
HTML coverage reports from `.profraw` files produced by Rust tests.

The workflow is intentionally split:
- Tests produce `.profraw` files (can run on host or target hardware).
- Reports are generated later on a host machine.

This makes it easy to collect coverage from cross-compiled tests or from
hardware-in-the-loop runs.

## Quick Start (Developers)

1) Run tests with coverage enabled:

```bash
bazel test --config=ferrocene-x86_64-linux --config=ferrocene-coverage \
  --nocache_test_results \
  //path/to:rust_tests
```

2) Generate coverage reports:

```bash
bazel run //:rust_coverage -- --min-line-coverage 80
```

The default report directory is:

```
$(bazel info bazel-bin)/coverage/rust-tests/<target>/blanket/index.html
```

The script prints per-target line coverage plus an overall summary line.

## Integrator Setup

### 1) MODULE.bazel

Add `score_tooling` and `score_toolchains_rust` as dependencies:

```starlark
bazel_dep(name = "score_tooling", version = "1.0.0")
bazel_dep(name = "score_toolchains_rust", version = "0.4.0")
```

### 2) .bazelrc

Add a Ferrocene coverage config. Names are examples; choose names that fit
your repo:

```
# Ferrocene toolchain for host execution
build:ferrocene-x86_64-linux --host_platform=@score_bazel_platforms//:x86_64-linux
build:ferrocene-x86_64-linux --platforms=@score_bazel_platforms//:x86_64-linux
build:ferrocene-x86_64-linux --extra_toolchains=@score_toolchains_rust//toolchains/ferrocene:ferrocene_x86_64_unknown_linux_gnu

# Coverage flags for rustc
build:ferrocene-coverage --@rules_rust//rust/settings:extra_rustc_flag=-Cinstrument-coverage
build:ferrocene-coverage --@rules_rust//rust/settings:extra_rustc_flag=-Clink-dead-code
build:ferrocene-coverage --@rules_rust//rust/settings:extra_rustc_flag=-Ccodegen-units=1
build:ferrocene-coverage --@rules_rust//rust/settings:extra_rustc_flag=-Cdebuginfo=2
build:ferrocene-coverage --@rules_rust//rust/settings:extra_exec_rustc_flag=-Cinstrument-coverage
build:ferrocene-coverage --@rules_rust//rust/settings:extra_exec_rustc_flag=-Clink-dead-code
build:ferrocene-coverage --@rules_rust//rust/settings:extra_exec_rustc_flag=-Ccodegen-units=1
build:ferrocene-coverage --@rules_rust//rust/settings:extra_exec_rustc_flag=-Cdebuginfo=2
test:ferrocene-coverage --run_under=@score_tooling//coverage:llvm_profile_wrapper
```

### 3) Add a repo-local wrapper target

In a root `BUILD` file:

```starlark
load("@score_tooling//coverage:coverage.bzl", "rust_coverage_report")

rust_coverage_report(
    name = "rust_coverage",
    bazel_configs = [
        "ferrocene-x86_64-linux",
        "ferrocene-coverage",
    ],
    query = 'kind("rust_test", //...)',
    min_line_coverage = "80",
)
```

Run it with:

```bash
bazel run //:rust_coverage
```

### 4) Optional: exclude known-problematic targets

```starlark
query = 'kind("rust_test", //...) except //path/to:tests',
```

## Cross/Target Execution

If tests run on target hardware, copy the `.profraw` files back to the host
and point the report generator to the directory:

```bash
bazel run //:rust_coverage -- --profraw-dir /path/to/profraw
```

## Running from an integration workspace (external labels)

You can invoke the report generator from a top-level integration repo (for
example, reference_integration) while targeting tests that live in external
modules. Use a query that references external labels and run the wrapper
target from the integration repo:

```bash
bazel run //images/linux_x86_64:per_rust_coverage --config=ferrocene-coverage -- \
  --query 'kind("rust_test", @score_persistency//src/rust/...)'
```

If the `.profraw` files were produced in that same workspace, the reporter
auto-discovers them under `bazel-testlogs/` (including
`bazel-testlogs/external/<repo>+` for external labels), so you do not need
to pass `--profraw-dir`. If they were copied from elsewhere, pass
`--profraw-dir` to point to the directory containing the `.profraw` files.
External source paths are resolved via Bazel's output_base so
`external/<repo>/...` paths are handled.

## Coverage Gate Behavior

`--min-line-coverage` applies per target. If any target is below the minimum,
the script exits non-zero so CI can fail the job. An overall summary is printed
for visibility but does not change gating behavior.

## Common Pitfalls

- **"running 0 tests"**: The Rust test harness found no `#[test]` functions,
  so coverage is 0%. Add tests or exclude the target from the query.
- **"couldn't find source file"** warnings: Usually path remapping or crate
  mapping issues. Check that `crate` attributes in `rust_test` targets point to
  the library crate (or exclude the target).
- **Cached test results**: Use `--nocache_test_results` if you need to re-run
  tests and regenerate `.profraw` files.

## Troubleshooting

### Coverage is 0% but tests ran
- Verify the target contains real `#[test]` functions. A rust_test target with
  no tests will run but report 0% coverage.
- Ensure you ran tests with `--config=ferrocene-coverage` so `.profraw` files
  exist.
- If the test binary is cached, use `--nocache_test_results`.

### "couldn't find source file" warnings
- Check `crate` mapping on `rust_test` targets. If `crate = "name"` is used,
  ensure it refers to the library crate in the same package.
- Confirm the reported paths exist in the workspace. Path remapping is required
  so `blanket` can resolve files under `--ferrocene-src`.

### No `.profraw` files found
- Ensure `test:ferrocene-coverage` sets `--run_under=@score_tooling//coverage:llvm_profile_wrapper`.
- Re-run tests with `--nocache_test_results`.
- If tests ran on target hardware, copy the `.profraw` files back and pass
  `--profraw-dir`.

### Coverage gate fails in CI
- The gate is per-target. A single target below the threshold fails the job.
- Use a stricter query (exclude known-zero targets) or add tests.

## CI Integration (Suggested Pattern)

Keep coverage generation separate from docs:

1) Coverage workflow:
   - run `bazel run //:rust_coverage`
   - upload `bazel-bin/coverage/rust-tests` as an artifact

2) Docs workflow:
   - download the artifact
   - copy into the docs output (e.g. `docs/_static/coverage/`)
   - publish Sphinx docs to GitHub Pages
