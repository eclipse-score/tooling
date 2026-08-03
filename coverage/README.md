<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Coverage — adoption guide

Reusable **LLVM source-based coverage pipeline** for S-CORE repositories:

- **One report for C++ and Rust** (line + branch coverage), produced by
  `llvm-cov` directly from covmap instrumentation — no gcov/genhtml.
- **Untested in-scope files appear at exact 0%** — since all targets are
  instrumented at build time, the reporter runs `llvm-cov --empty-profile`
  over the archives of libraries no test links against. No heuristics; the
  line/branch denominators come from the compiler's own coverage map.
- **Justification system**: `COV_JUSTIFIED` in-code markers + a YAML database
  turn intentionally-uncovered lines into *justified* lines, tracked in an
  **effective coverage** metric with stale-justification detection.
- **Gating**: the report generator exits non-zero when effective coverage is
  below `COVERAGE_THRESHOLD` (default 100).

How the pipeline works internally is documented in
[COVERAGE_GUIDE.md](COVERAGE_GUIDE.md). A complete, working consumer setup is
the [integration_tests/](integration_tests/) workspace — every snippet below
is copied from it.

## Components

| Target / file | Purpose |
|---|---|
| `@score_tooling//coverage:merger` | Per-test coverage output generator (profraw → profdata + object metadata). Referenced directly from your bazelrc. |
| `@score_tooling//coverage:reporter` | Final report generator (merged profdata → HTML + LCOV + text summary). Not referenced directly — wrapped by `score_coverage_reporter`. |
| `defs.bzl :: score_coverage_scope` | Declares WHICH targets are in scope; emits the source allowlist + baseline-archive manifest via an aspect. |
| `defs.bzl :: score_coverage_reporter` | Consumer-side wrapper wiring your scope, workspace root and LLVM tools into the reporter. |
| `@score_tooling//coverage:generate_coverage_html` | Orchestration: unpacks the report, runs justifications, enforces the threshold, optionally archives. |
| `@score_tooling//coverage:justify` | Parses the justification YAML + in-code markers into a manifest. |
| `@score_tooling//coverage:effective_coverage` | Post-processes the HTML: restyles justified lines, computes effective coverage, detects stale justifications. |
| `@score_tooling//coverage:enable_llvm_coverage_for_death_tests` | `cc_feature` adding `-mllvm -runtime-counter-relocation` (continuous-mode profiling for death tests). |

## Prerequisites

1. A Bzlmod workspace (`MODULE.bazel`).
2. Linux x86_64 host (the pipeline runs on the host platform; do not combine
   with QNX/cross platform configs).
3. For Rust: a Ferrocene toolchain built by `ferrocene_toolchain_builder`
   **>= 1.3.1** (its coverage-tools tarball ships `llvm-cov`/`llvm-profdata`
   built from the same LLVM as rustc) wired through `score_toolchains_rust`
   **>= 0.10.0**.

## 1. Depend on score_tooling

```starlark
bazel_dep(name = "score_tooling", version = "<version>")
```

Add one line to your **root** `BUILD` file so the reporter can locate your
workspace root at runtime:

```starlark
exports_files(["MODULE.bazel"])
```

## 2. Declare the coverage toolchains (MODULE.bazel)

```starlark
bazel_dep(name = "score_toolchains_rust", version = "0.10.0", dev_dependency = True)
bazel_dep(name = "toolchains_llvm", version = "1.8.0", dev_dependency = True)

llvm = use_extension("@toolchains_llvm//toolchain/extensions:llvm.bzl", "llvm", dev_dependency = True)
llvm.toolchain(
    cxx_standard = {"": "c++17"},
    extra_known_features = [
        "@score_tooling//coverage:enable_llvm_coverage_for_death_tests",
    ],
    llvm_version = "22.1.7",
    stdlib = {"": "stdc++"},
)
use_repo(llvm, "llvm_toolchain", "llvm_toolchain_llvm")
```

For Rust, additionally instantiate a Ferrocene toolchain **with coverage
tools** (see [integration_tests/MODULE.bazel](integration_tests/MODULE.bazel)
for the full block — `coverage_tools_url`/`sha256` from the
ferrocene_toolchain_builder 1.3.1 release):

```starlark
llvm_ferrocene = use_extension(
    "@score_toolchains_rust//extensions:ferrocene_toolchain_ext.bzl",
    "ferrocene_toolchain_ext",
    dev_dependency = True,
)
llvm_ferrocene.toolchain(
    name = "ferrocene_x86_64_unknown_linux_gnu_llvm",
    coverage_tools_url = "...",     # coverage-tools tarball, builder >= 1.3.1
    coverage_tools_sha256 = "...",
    ...
)
use_repo(llvm_ferrocene, "ferrocene_x86_64_unknown_linux_gnu_llvm")
```

rules_rust only instruments crates when the `rust_toolchain` declares
`llvm_cov` — a Ferrocene instance *without* `coverage_tools_url` silently
produces no Rust coverage.

## 3. Declare scope and reporter (BUILD)

In e.g. `tools/coverage/BUILD`:

```starlark
load("@score_tooling//coverage:defs.bzl", "score_coverage_reporter", "score_coverage_scope")

score_coverage_scope(
    name = "coverage_scope",
    testonly = True,
    deps = [
        "//src/mylib",           # cc_library
        "//src/rust/mycrate",    # rust_library
        "//src/rust/tool:tool",  # rust_binary
    ],
)

score_coverage_reporter(
    name = "reporter_wrapper",
    testonly = True,
    coverage_scope = ":coverage_scope",
    llvm_cov = "@llvm_toolchain//:llvm-cov",
    llvm_profdata = "@llvm_toolchain//:llvm-profdata",
    llvm_cxxfilt = "@llvm_toolchain_llvm//:bin/llvm-cxxfilt",
)
```

The scope aspect walks the listed targets and their transitive in-workspace
deps, collecting source files (allowlist) and compiled archives (baselines).
Everything in scope but untested shows up at 0%; everything outside the scope
(tests, mocks, external deps) is filtered out of the report.

## 4. Import the bazelrc config

Copy the `coverage:llvm_cov` block from
[integration_tests/.bazelrc](integration_tests/.bazelrc) into your
repository's bazelrc (directly or via `import`). The two labels to adapt:

```
coverage:llvm_cov --coverage_output_generator=@score_tooling//coverage:merger
coverage:llvm_cov --coverage_report_generator=//tools/coverage:reporter_wrapper
```

The merger reference points into score_tooling as-is; the reporter_wrapper
label is the target you declared in step 3.

> **Do NOT combine `--config=llvm_cov` with configs that append other
> `--extra_toolchains`** (e.g. a GCC host config): the last toolchain wins
> resolution and a GCC toolchain produces no covmap data.

## 5. (Optional) Set up justifications

`tools/coverage/coverage_justifications.yaml`:

```yaml
version: 1
justifications:
  - id: hw-unreachable-on-x86
    category: platform_specific
    platforms: [linux]
    reason: |
      ARM-only error path; cannot be exercised by x86 CI.
```

Mark the code in place:

```cpp
return false;  // COV_JUSTIFIED hw-unreachable-on-x86

// or a region:
// COV_JUSTIFIED_START hw-unreachable-on-x86
if (running_on_arm()) { ... }
// COV_JUSTIFIED_STOP
```

Valid categories: `defensive_programming`, `tool_false_positive`,
`platform_specific`, `other`. IDs are kebab-case. Justified lines render
orange in the HTML and count as covered in the *effective* metric; a
justification on a line that is meanwhile covered is flagged as **stale**.

## 6. Run it

```bash
bazel coverage --config=llvm_cov //... --build_tests_only

bazel run @score_tooling//coverage:generate_coverage_html -- \
    --yaml tools/coverage/coverage_justifications.yaml

# CI variant: archive HTML + LCOV + JUnit XMLs, gate at 95%:
COVERAGE_THRESHOLD=95 bazel run @score_tooling//coverage:generate_coverage_html -- \
    --yaml tools/coverage/coverage_justifications.yaml \
    --archive coverage_artifacts
```

`--build_tests_only` matters: without it, coverage builds (not runs) every
target matched by the pattern, including e.g. `manual`-tagged or
platform-incompatible test binaries.

## Customization knobs

| Need | Knob |
|---|---|
| Different gate | `COVERAGE_THRESHOLD=<pct>` env var (default 100; exit 1 below) |
| Output directory | positional `output-dir` argument (default `coverage_<platform>`) |
| Platform-specific justifications | `--platform linux\|qnx` (default linux) |
| JUnit XMLs subtree in the archive | `--testlogs-subdir <dir>` (default: whole `bazel-testlogs`) |
| Different LLVM version | your own `llvm.toolchain(...)`; pass its labels in step 3 |
| Rust branch coverage | `-Zcoverage-options=branch` (needs a nightly-based/rolling Ferrocene; drop the flag on stable) |

## Troubleshooting

| Symptom | Cause |
|---|---|
| `... is not the LLVM pipeline zip report` | The coverage run used the default lcov path — the `--config=llvm_cov` flags (or your bazelrc import) were not active. |
| No `.rs` files in the report | The Ferrocene toolchain in use has no `llvm_cov` attached (missing `coverage_tools_url`, or a non-coverage toolchain instance won resolution). |
| No C++ files / empty covmap | A GCC toolchain won toolchain resolution — check for conflicting `--extra_toolchains` from another config. |
| `Neither __llvm_profile_counter_bias nor ...` in test logs, no profraw | Continuous mode without runtime counter relocation: the `enable_llvm_coverage_for_death_tests` feature (C++) or the `-Cllvm-args=-runtime-counter-relocation` rustc flag is missing. |
| `no coverage data found` on a Rust archive | Handled automatically (rlib expansion); if you see it, the reporter predates the rlib fix. |
| `error[E0463]: can't find crate for profiler_builtins` | The Ferrocene sysroot lacks profiler_builtins (builder < 1.3.1, or a miri sysroot leaked into coverage builds). |
| `the following arguments are required: --workspace_root` | You pointed `--coverage_report_generator` at `:reporter` directly instead of your `score_coverage_reporter` target. |
| Coverage numbers differ between runs on identical code | Dynamic-linking instrumentation clash — ensure `--dynamic_mode=off` from the bazelrc block is active. |

## Migration from the removed Ferrocene symbol-report/blanket flow

`rust_coverage_report`, `//coverage:ferrocene_report` and its helper scripts
were removed. Replace:

- `bazel run //:rust_coverage` → steps 1–6 above (one report for both
  languages, exact untested-file entries, justifications, effective gate).
- `test:ferrocene-coverage --run_under=@score_tooling//coverage:llvm_profile_wrapper`
  is no longer needed — Bazel's own coverage collection sets
  `LLVM_PROFILE_FILE`. The wrapper target still exists for repositories that
  have not migrated yet.

---

## Repository-internal: Combined Rust + Python Coverage

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

