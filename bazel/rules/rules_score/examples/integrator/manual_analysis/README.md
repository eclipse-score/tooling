<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Integrator Manual Analysis Reviews

The integrator reviews two received AoUs directly. Both committed reviews are
expected to fail: the available sources do not demonstrate that the deployment
contracts are satisfied. This records missing evidence, not a proven violation.

| Review | Received AoU | Unresolved evidence |
| --- | --- | --- |
| `received_shared_memory` | `SampleType.SampleAoU@1` | Shared-memory creation paths, naming validation, and deployment configuration |
| `received_timing` | `OtherLibrary.TimingConstraint@1` | Timer configuration, scheduling, and worst-case timing for the cyclic call |

Run from the integrator example root:

```sh
bazel test //manual_analysis/received_shared_memory:review --test_output=errors
bazel test //manual_analysis/received_timing:review --test_output=errors
```

Both tests fail on the committed negative assertion even when their locks are
current. The targets are tagged `manual`, so `bazel test //...` skips these review
tests. The DE still consumes their generated LOBSTER evidence.

The example uses `maturity = "development"`, so the DE test may pass with
warnings. A complete traceability link or passing DE test does not close either
AoU. Explicit review tests check both lock freshness and the recorded assertion.

To close a review, obtain the missing evidence and repeat the interactive review:

```sh
bazel run //manual_analysis/received_shared_memory:review.update
```

Use `received_timing` for the timing review. Commit `results.json` and
`manual.lock` together after reviewing the findings. Do not change the answer or
refresh the lock merely to make a failing review pass.
