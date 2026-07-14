<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Validation

## Unit Tests

Unit tests in `rules_score` components are written with **GoogleTest** and built
with `cc_test`. Each test case that covers a component requirement must carry
a `lobster-tracing` annotation so that the build can link the test back to the
requirement.

### Annotating Tests

Call `RecordProperty` inside the test body next to the respective code blocks:

```cpp
TEST_F(MyFixture, DoesXWhenY) {
    ::testing::Test::RecordProperty("lobster-tracing", "MessagePassing.OsIpcFaultHandling");
    ::testing::Test::RecordProperty("given",  "a connected client");
    ::testing::Test::RecordProperty("when",   "the OS IPC call fails");
    ::testing::Test::RecordProperty("then",   "the client receives an error");
}
```

| Property | Required | Description |
|---|---|---|
| `lobster-tracing` | yes | Comma-separated requirement IDs; links the test to one or more `CompReq` records |
| `given` | no | Initial state / precondition |
| `when` | no | Action or event under test |
| `then` | no | Expected outcome |

A test without `lobster-tracing` has no traceability and is not included in
coverage tracking.

### Stating Coverage for a requirement

Coverage is declared through a committed `coverage.lock.yaml` file that lists,
per requirement, every test case (uid + given/when/then) that covers it.
Committing the file is stating the coverage claim.

In Bazel the yaml file can be linked to the `component` macro via the `coverage_lock` attribute:

```starlark
component(
    name = "my_component",
    requirements = [":my_component_requirements"],
    components  = [":unit_a", ":unit_b"],
    coverage_lock = "coverage.lock.yaml",
)
```

Two complementary workflows keep the lock file in sync:

- **`bazel run …update`** — reads the current test results and **rewrites**
  `coverage.lock.yaml` in the source tree.
- **`bazel test //…`** — the build action recomputes coverage from the same
  test results and **compares** it against the committed lock. Any drift (new
  test, removed test, changed GWT text, version bump) fails the build until the
  lock is refreshed and re-committed.

For the full tool description — lock file format, update workflow, design
decisions — see {doc}`../tool_reference/req_coverage`.
