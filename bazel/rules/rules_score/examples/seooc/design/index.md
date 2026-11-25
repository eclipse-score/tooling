<!-- ----------------------------------------------------------------------------
  Copyright (c) 2025 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Safety Software SEooC Example — Static Design

This is a **compose** example (see `architectural_design.rst`'s "Authoring
Pages Alongside Diagrams"): this file is named `index.md`, so its content is
rendered above the generated navigation for this directory instead of
replacing it — `static_design` below keeps its own entry.

## Zoom into a black box

The dynamic view shows one call, `GetNumber()` from `App`, at three levels
(`get_number_seooc`, `get_number_component`, `get_number_unit` below). Each
level repeats the call with the same sender and method, addressed to a direct
child of the previous callee, as the first call of its diagram. The diagrams
carry no marker; the validation links them by participant uid.

| Level | Call | What the level adds |
|-------|------|---------------------|
| SEooC | `App -> safety_software_seooc_example : GetNumber()` | the black box and its exit `Log()` |
| Component | `App -> component_example : GetNumber()` | `component_example -> unit_1 : GetNumber()` |
| Unit | `App -> unit_1 : GetNumber()` | `unit_1 -> unit_2 : GetData()` |

The `architectural_design` validation checks that:

- every call the black box makes to its environment (`Log()` to `Platform`)
  appears in at least one deeper diagram; the component and the unit level keep
  it,
- a deeper diagram uses no architecture element outside the refined one,
  except the sender and the partners the overview shows, and not the refined
  one itself,
- the zoomed call, when synchronous (`->`), is closed with `-->`, `return`
  (after `++` or `activate`) or `deactivate`; an asynchronous call (`->>`)
  needs no reply.

Removing `unit_1 -> Platform : Log()` from `get_number_unit.puml` fails the
build with `ZoomExitMissing`. A call without a deeper diagram stays a black box
and is not checked.

In the rendered pages the label `GetNumber()` of the SEooC and component
diagrams is a link to the next deeper diagram; the unit level has none.
