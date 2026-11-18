<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Integrator Manual Analysis Review

The integrator reviews `OtherLibrary.TimingConstraint@1` directly. The
committed review is expected to fail: the available sources do not demonstrate
that the timing contract is satisfied. This records missing evidence, not a
proven violation. `SampleType.SampleAoU@1`, `OtherLibrary.SpaceConstraint@1` and
`OtherLibrary.ValidInputPointer@1` are covered by component requirements
`COMP_INT_001` and `COMP_INT_005` instead.

| Review | Received AoU | Unresolved evidence |
| --- | --- | --- |
| `received_timing` | `OtherLibrary.TimingConstraint@1` | Timer configuration, scheduling, and worst-case timing for the cyclic call |

Run from the integrator example root:

```sh
bazel test //manual_analysis/received_timing:review --test_output=errors
```

The test fails on the committed negative assertion even when the lock is
current. The target is tagged `manual`, so `bazel test //...` skips it. The DE
still consumes its generated LOBSTER evidence.

The example uses `maturity = "development"`, so the DE test may pass with
warnings. A complete traceability link or passing DE test does not close the
AoU. The review test checks both lock freshness and the recorded assertion.

To close the review, obtain the missing evidence and repeat the interactive
review:

```sh
bazel run //manual_analysis/received_timing:review.update
```

Commit `results.json` and `manual.lock` together after reviewing the findings.
Do not change the answer or refresh the lock merely to make the review pass.
