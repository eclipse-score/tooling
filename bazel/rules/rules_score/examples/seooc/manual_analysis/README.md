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

Both review targets are tagged `manual`, so wildcard test selections such as
`bazel test //...` and `bazel test //manual_analysis/...` skip both review tests.
Run each review explicitly using its target label, as shown below. The DE still
consumes both reviews' generated LOBSTER evidence; the tag does not disable
artifact generation through explicit dependencies.

## Input Pointer Review

`received_input_pointer` reviews `OtherLibrary.ValidInputPointer@1` through
`SampleComponentExtra.REQ_COMP_EXTRA_004@1`. It checks all `unit_1` call sites
and the dependency contract and implementation: `Foo::GetNumber` passes a
call-local const uint8_t to `OtherLibraryAPI::Compute`, which reads synchronously
without retaining the pointer. A returned-value test alone would not prove the
object's lifetime, so this requirement uses source-review evidence instead.

The committed source-review evidence was prepared by the coding assistant and is
not an independent human safety approval. Run the interactive review to record
your own findings.

## Space Constraint Review (Expected Failure)

`received_space_constraint` retains the original `OtherLibrary.SpaceConstraint@1`
and reviews its local allocation, `REQ_COMP_EXTRA_003@1`. The source review finds
no dynamic allocation in Foo or Compute, but that is not a complete memory-budget
proof. The original AoU does not define the accounting boundary or unit precisely,
and deployment/concurrency evidence is missing. Its recorded conclusion is
therefore **not demonstrated**, not a measured budget violation.

`Foo.FootprintWithinBudget` checks only that `sizeof(Foo)` is below 10 MiB for
the current build. It does not prove total runtime memory usage or resolve the
AoU's MB/MiB ambiguity.

Explicitly running `//manual_analysis/received_space_constraint:review` still
fails on the committed negative assertion, even after its lock is refreshed.
To close it, clarify the contract with its owner, obtain the relevant evidence,
and repeat the review. Do not merely refresh the lock or change the answer.

## Review Context

| Context target | Review inputs |
| --- | --- |
| `//unit_1:manual_analysis_context` | C++ sources, transitive dependency headers, and selected rule attributes |
| `//manual_analysis:received_aou_requirements_context` | `docs/requirements/component_requirements_extra.trlc` |
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

Replace `received_input_pointer` with `received_space_constraint` to run the
memory review; its committed result is expected to fail.

Review and commit each review's `results.json` and `manual.lock` together. Do not
refresh the lock merely to dismiss a changed-input failure. The test does not
independently prove that the reviewer's reasoning is correct.

The DE includes this evidence through `dependable_element.manual_analysis`:

```sh
bazel test //:safety_software_seooc_example --test_output=errors
```

The example uses `maturity = "development"`, so the DE test may pass with warnings.
Neither a passing DE test nor a green traceability card closes the memory AoU;
traceability completeness is not verification success.
