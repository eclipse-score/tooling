<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Unified Code Coverage (LLVM) — how the pipeline works

This document explains, from first principles, how the unified Rust + C++
coverage pipeline in `@score_tooling//coverage` works: the mechanism, the
module/consumer split, the non-obvious pitfalls, and where the pipeline comes
from. For the step-by-step consumer setup see [README.md](README.md); for a
working consumer workspace see [integration_tests/](integration_tests/).

---

## 1. Background concepts

### 1.1 What "code coverage" means here

When we run the test suite, we want to know **which lines of production code
were actually executed**. The compiler helps: it can *instrument* the code —
insert tiny counters at every branch and statement. When an instrumented test
binary runs, it writes the counter values to a file. Tooling then maps those
counters back to source lines and produces a report: green = executed,
red = never executed.

### 1.2 Bazel in three paragraphs

Bazel is the build system used by S-CORE. Code is organized into **targets**
(a library, a binary, a test), declared in files named `BUILD`. Targets
reference each other by **labels** like `//src/rust/mycrate:tests`
(`//path/to/package:target_name`).

External dependencies (compilers, libraries) are declared in `MODULE.bazel`.
A **toolchain** is Bazel's packaging of a compiler + flags; you can register
several and select one per build. Command-line defaults live in `.bazelrc`,
grouped into named **configs**: `bazel test --config=foo` applies all lines
starting with `test:foo`.

Bazel has a built-in coverage mode: `bazel coverage <targets>` builds the
targets with instrumentation, runs the tests, and post-processes the results.
Two hooks matter for us: `--coverage_output_generator` (a tool that processes
each test's raw coverage output) and `--coverage_report_generator` (a tool
that combines everything into the final report). **This pipeline replaces
both with its own tools** — that is its core.

### 1.3 Rust in two paragraphs

Rust code is organized into **crates** (≈ libraries/binaries). The Rust
compiler is `rustc`; S-CORE uses **Ferrocene**, a safety-certified Rust
toolchain distribution. Bazel builds Rust via the `rules_rust` plugin.

Crucially, `rustc` is built on the same compiler backend as Clang (**LLVM**).
That means Rust and C++ can use the *same* coverage instrumentation format —
which is what makes a single unified report possible.

### 1.4 The LLVM coverage toolchain

The pipeline is built on LLVM's "source-based coverage":

| Artifact | Produced by | Contains |
|---|---|---|
| instrumented binary | Clang (C++) / rustc (Rust) with coverage flags | counters + a "covmap" mapping counters → source lines |
| `.profraw` | running the instrumented test | raw counter values |
| `.profdata` | `llvm-profdata merge` | merged, indexed counters |
| HTML / LCOV / text report | `llvm-cov show/export/report` | human- and machine-readable coverage |

---

## 2. How the pipeline works

There are two phases.

### 2.1 Phase 1 — collection (`bazel coverage`)

```
bazel coverage --config=llvm_cov //... --build_tests_only
```

The `llvm_cov` config (the consumer copies it from
[integration_tests/.bazelrc](integration_tests/.bazelrc)) does four things:

1. **Swaps the compilers.** C++ is compiled with a hermetic Clang/LLVM
   toolchain (`@llvm_toolchain`) instead of GCC; Rust with a Ferrocene
   toolchain that has LLVM coverage tools attached — wired in automatically
   by score_toolchains_rust >= 0.9.2 from the coverage-tools tarball, see §5.
   Both emit the same covmap format.
2. **Turns on instrumentation.** `--experimental_use_llvm_covmap` plus the
   `coverage` feature for C++; `rules_rust` adds `-Cinstrument-coverage` to
   rustc automatically once the toolchain declares coverage tools. An extra
   flag (`-Cllvm-args=-runtime-counter-relocation`) enables "continuous
   mode" so coverage survives even if a test terminates abnormally (same
   purpose as the `enable_llvm_coverage_for_death_tests` cc_feature on the
   C++ side). **Branch coverage** needs one more flag per language: Clang
   emits branch regions by default, but rustc only does so with
   `-Zcoverage-options=branch` — an unstable option that works on
   *rolling* (nightly-based) Ferrocene builds. On a stable-channel
   Ferrocene the flag must be dropped (Rust branch columns revert to `-`)
   until rustc stabilizes it.
3. **Installs the per-test tool**
   (`--coverage_output_generator=@score_tooling//coverage:merger`). After
   each test runs, `merger.py` finds the test's `.profraw` files, merges
   them into one `.profdata` with `llvm-profdata`, records which
   instrumented binary was involved, and zips both up as the test's
   `coverage.dat`.
4. **Installs the final tool** (`--coverage_report_generator=` the
   consumer's `score_coverage_reporter` target). After all tests finish,
   `reporter.py` merges every per-test `.profdata` into one, then runs
   `llvm-cov` three times: `show` → HTML report, `export` → LCOV data (for
   dashboards), `report` → text summary. All three are zipped into
   `bazel-out/_coverage/_coverage_report.dat`.

**Scope — which files appear in the report.** LLVM covmap instruments
*everything*, including test code and third-party libraries. Filtering
happens at report time using an **allowlist** generated by the consumer's
`score_coverage_scope` target (`coverage_scope.bzl`): an aspect walks the
dependency graph starting from the listed production targets and collects
every in-workspace source file they own. The reporter excludes everything
else (test sources, googletest, external deps, ...).

**Baseline — files with no tests at all.** A file that no test executes
produces no coverage data, so naive tooling silently omits it — it looks
like there is no problem when in fact coverage is 0%. The scope aspect
therefore also collects the compiled libraries/binaries (`.a` archives for
cc_library/rust_library, the coverage-built executable for rust_binary), and
the reporter runs `llvm-cov --empty-profile` over them so untested files
show up with **exact** 0% line and branch entries — the denominators come
from the compiler's own coverage map, not from any source-text heuristic.
Rust rlib archives need special handling here: their leading `lib.rmeta`
member makes llvm-cov reject the whole archive, so the reporter expands them
into their `.o` members first.

### 2.2 Phase 2 — report generation & gating

```
bazel run @score_tooling//coverage:generate_coverage_html -- \
    --yaml tools/coverage/coverage_justifications.yaml
```

`generate_coverage_html.sh` unpacks the HTML from the zip, then applies the
**justification system**:

- `justify.py` reads the consumer's `coverage_justifications.yaml` plus
  in-code markers and produces a manifest of "argued" lines. A justification
  says: *this line cannot reasonably be covered by a test, and here is why*
  (e.g. defensive code for conditions that cannot occur). Markers work in
  both languages:

  ```rust
  unreachable!();  // COV_JUSTIFIED my-justification-id
  ```
  ```cpp
  default: return Error;  // COV_JUSTIFIED my-justification-id
  ```

  Every marker id must exist in the YAML with a category and a written
  reason — a justification is a reviewed engineering argument, not an
  opt-out.

- `effective_coverage.py` recolors justified lines **orange** in the HTML
  (with the reason as tooltip) and computes:

  ```
  raw coverage       = covered / total
  effective coverage = (covered + justified) / total
  ```

  It also flags **stale** justifications — lines that are justified but
  meanwhile covered by a test — so the database stays clean.

- Finally the script compares effective line coverage against the
  `COVERAGE_THRESHOLD` environment variable (default **100**) and **fails
  (exit 1)** when below. The model: every uncovered line must eventually be
  either tested or justified; the threshold is ratcheted up as gaps close.

### 2.3 Day-to-day commands

```bash
# collect coverage (Rust + C++, one run)
bazel coverage --config=llvm_cov //... --build_tests_only

# report without gating
COVERAGE_THRESHOLD=0 bazel run @score_tooling//coverage:generate_coverage_html -- \
    --yaml tools/coverage/coverage_justifications.yaml

# open it
xdg-open coverage_linux/index.html

# CI-style archive (HTML + LCOV + justification report + JUnit XMLs)
bazel run @score_tooling//coverage:generate_coverage_html -- \
    --yaml tools/coverage/coverage_justifications.yaml --archive my-report
```

> **Do not** combine `--config=llvm_cov` with configs that register other
> C++ toolchains (e.g. a GCC host config): the last `--extra_toolchains`
> wins resolution, GCC cannot produce covmap data, and the report script
> fails loudly on the resulting non-zip report.

---

## 3. The module/consumer split

Almost everything in the pipeline is generic. What is repo-specific is
exactly three things: **(a)** the list of production targets in the scope,
**(b)** the justification YAML, **(c)** the toolchain pins in MODULE.bazel.
The split follows directly:

**Lives in `@score_tooling//coverage` (shared):**

| File | Role |
|---|---|
| `merger.py` | per-test profraw → profdata (C++ `objects_list.txt` and Rust ELF-manifest discovery) |
| `reporter.py` | final merge + llvm-cov show/export/report + allowlist filtering + `--empty-profile` baselines + rlib expansion |
| `coverage_scope.bzl` | the scope aspect/rule (CcInfo + CrateInfo) |
| `reporter_wrapper.bzl` + `defs.bzl` | the consumer-facing `score_coverage_scope` / `score_coverage_reporter` API |
| `justify.py`, `effective_coverage.py`, `generate_coverage_html.sh` | justification + gating layer |
| `enable_llvm_coverage_for_death_tests` | cc_feature for continuous-mode profiling |

**Lives in the consumer repository:**

| Piece | Why it cannot move |
|---|---|
| `score_coverage_scope(deps = [...])` | names the repo's production targets |
| `score_coverage_reporter(...)` | carries the repo's LLVM tool labels and workspace root |
| `coverage_justifications.yaml` | reviewed, repo-specific engineering arguments |
| MODULE.bazel toolchain blocks | LLVM + Ferrocene pins are per-repo decisions |
| the `coverage:llvm_cov` bazelrc block | bazelrc cannot be imported across modules; copied from the canonical snippet |

Two wiring details make the external hosting work, both easy to get wrong:

- **Runfiles paths span repositories.** The reporter wrapper mixes files
  from `_main` (the consumer), `score_tooling` and toolchain repos, so every
  path in the generated launcher uses rlocation form (`../repo/...` →
  `repo/...`), and the launcher derives its own `RUNFILES_DIR` from `$0` —
  the inherited value points at the *test's* runfiles tree, not ours.
- **The baseline manifest lists consumer files.** The reporter resolves
  manifest entries against `_main` explicitly; using the runfiles library's
  "current repository" would resolve against `score_tooling` and find
  nothing.

---

## 4. Non-obvious pitfalls (why these lines exist)

These are the "landmines" discovered while bringing the pipeline up in
communication and persistency:

1. **Warnings-as-errors under Clang.** Repos whose deps request the
   `treat_warnings_as_errors` feature may need
   `--features=-treat_warnings_as_errors` **and**
   `--host_features=-treat_warnings_as_errors` in the coverage config:
   Clang emits warnings GCC doesn't, and Bazel builds the coverage reporter
   (and hence the scope's libraries) a second time "as a tool" in a separate
   configuration — that's what the `--host_features` variant covers.
2. **Never disable the `coverage` feature** under the LLVM toolchain — it
   *is* the instrumentation (`-fprofile-instr-generate
   -fcoverage-mapping`).
3. **`-Cllvm-args=-runtime-counter-relocation`** for Rust — without it,
   continuous-mode profiling errors out and Rust tests write no `.profraw`.
4. **`llvm-cov report` prints raw covmap paths** (`/proc/self/cwd/...`) —
   `--path-equivalence` does not rewrite *displayed* paths; the reporter
   normalizes them, otherwise the allowlist silently excludes all C++
   files.
5. **The Rust toolchain lists its own `llvm-cov`/`llvm-profdata` binaries as
   coverage metadata** — the merger must skip `external/` entries or the
   final merge emits "mismatched data" warnings.
6. **Rust branch coverage is opt-in and channel-dependent** — llvm-cov only
   renders branch data that the compiler wrote into the covmap; stable rustc
   writes none. `-Zcoverage-options=branch` enables it on nightly-based
   toolchains (like the Ferrocene rolling build). Verify with
   `llvm-cov export`: the `branches` arrays must be non-empty for Rust
   files.

---

## 5. Toolchain provisioning (score_toolchains_rust + ferrocene_toolchain_builder)

Solved at the source, no consumer configuration needed:

- `ferrocene_toolchain_builder` >= **1.3.1** ships `llvm-cov`,
  `llvm-profdata` and `llvm-cxxfilt` in the coverage-tools tarball, built
  from the same LLVM tree as rustc (so the tools can always read the
  profraw/covmap the compiler emits). 1.3.1 also rebuilds ALL artifacts from
  a single tree — toolchain tarballs, miri-sysroots (now including
  `libprofiler_builtins`) and coverage tools are ABI-consistent.
- `score_toolchains_rust` >= **0.9.2** (current: 0.10.0) auto-wires the
  tools into the generated `rust_toolchain` whenever the coverage-tools
  tarball contains them. rules_rust then instruments crates under
  `bazel coverage` and exports `RUST_LLVM_COV`/`RUST_LLVM_PROFDATA` to the
  coverage runner.

Consequently consumers need no coverage-specific Rust toolchain at all: the
**standard** toolchains declared in score_toolchains_rust's own MODULE.bazel
(e.g. `@score_toolchains_rust//toolchains/ferrocene:ferrocene_x86_64_unknown_linux_gnu`)
already pin the coverage-tools tarball, so the toolchain registered for
regular builds is the one that produces coverage — same rustc, same LLVM.

---

## 6. Origins and validation evidence

The pipeline core was built in the `communication` repository
(eclipse-score/communication, Rust support merged with PR #772 and visible
in its nightly coverage reports), then ported to `persistency` and finally
centralized here. During the persistency port, the previous Rust mechanism
(Ferrocene `symbol-report` + `blanket`, per-target 90% gate — the flow this
module's removed `rust_coverage_report` rule drove) was run against the new
pipeline **on the same commit**. Percentages agreed within a few points on
most files (the two tools attribute lines differently: blanket is
symbol-oriented, llvm-cov counts every executable region), with one headline
difference:

> A 311-line, 0%-covered Rust binary (`kvs_tool.rs`, no tests) was
> **invisible** to the old pipeline while its 90% gate passed. The new
> baseline mechanism reports it at exact 0% and the effective-coverage gate
> accounts for it.

That gap — untested files silently missing from reports — is the main
correctness argument for this pipeline, alongside unified C++ + Rust
reporting and branch coverage for Rust.

Planned next step for the ecosystem: a reusable GitHub Actions workflow in
`eclipse-score/cicd-workflows` wrapping the collection + report + artifact
steps, so consumer repos add one `uses:` block instead of a hand-written
job.
