<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

<!--
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
-->

# Software Unit Design

The `unit_design` rule documents the **internal implementation** of a single software unit — how its source code is structured, what data flows through it, and how it behaves at the code level. This is distinct from the higher-level architectural design diagrams (see {doc}`architectural_design`), which describe the intended component structure of the SEooC as a whole.

A `unit_design` target is referenced by a `unit` target (see {doc}`architectural_design` — *Implementation Architecture in Bazel*) to attach code-level design artefacts to the unit.

## `unit_design` — Code-Level Design Diagrams

The `unit_design` rule attaches PlantUML diagrams to a unit. It uses the same `static` / `dynamic` category split as `architectural_design`, but scoped to a single unit's implementation.

### `unit_design` Rule Reference

For the complete `unit_design` attribute reference, see {ref}`unit_design <rule-unit-design>` in the rule index.
