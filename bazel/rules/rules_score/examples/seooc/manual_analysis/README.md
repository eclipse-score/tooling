<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Manual Analysis Reviews for the SEooC Example

Each review is an independent Bazel package with its own BUILD file defining a
`review` target, alongside `analysis.yaml`, `results.json`, and `manual.lock`.
See the [Manual Analysis guide](../../../../../../manual_analysis/README.md)
for general setup and usage.

The review target is a regular test, so `bazel test //...` runs it and fails
when the lock is out of date or the recorded assertion is negative. The DE
consumes the review's generated LOBSTER evidence; tracing alone does not mean
the review passed.

## Input Pointer Review

`received_input_pointer` reviews `OtherLibrary.ValidInputPointer@1` directly.
The review checks all `unit_1` call sites
and the dependency contract and implementation: `Foo::GetNumber` passes a
call-local const uint8_t to `OtherLibraryAPI::Compute`, which reads synchronously
without retaining the pointer. A returned-value test alone would not prove the
object's lifetime, so this AoU uses source-review evidence instead.

The committed evidence is example data, not a safety approval. Run the
interactive review to record your own findings.

`OtherLibrary.SpaceConstraint@1` is not reviewed here: the DE covers it through
component requirement `REQ_COMP_EXTRA_003`. The evidence behind it is
placeholder: `Foo.FootprintWithinBudget` only checks `sizeof(Foo)` and neither
proves a runtime memory bound nor resolves the AoU's MB/MiB ambiguity.

## Review Context

| Context target | Review inputs |
| --- | --- |
| `//unit_1:manual_analysis_context` | C++ sources, transitive dependency headers, and selected rule attributes |
| `//manual_analysis:dependency_contract_context` | Dependency AoU and API description |

The lock covers these inputs, the analysis procedure, results, and selected C++
rule attributes, including transitive implementation inputs. It does not capture
entire BUILD files, full toolchain settings, or explicit dependency edges. Any
change within an included file invalidates the lock and requires impact review.

## Run the Review

Run from the SEooC example root, the parent of this directory. Use `.update` to
perform the interactive review and refresh its results and lock, then test:

```sh
bazel run //manual_analysis/received_input_pointer:review.update
bazel test //manual_analysis/received_input_pointer:review --test_output=errors
```

Review and commit the review's `results.json` and `manual.lock` together. Do not
refresh the lock merely to dismiss a changed-input failure. The test does not
independently prove that the reviewer's reasoning is correct.

The DE includes this evidence through `dependable_element.manual_analysis`:

```sh
bazel test //:safety_software_seooc_example --test_output=errors
```

The example uses `maturity = "development"`, so the DE test may pass with
warnings. Traceability completeness is not verification success.
