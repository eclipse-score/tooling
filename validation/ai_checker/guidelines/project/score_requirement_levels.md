<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# SCORE Requirement Levels

Project-specific requirement levels for the SCORE Requirements Engineering
Process. These complement the general requirements guidelines.

| Level | Scope | Derived From |
|---|---|---|
| **Stakeholder Requirement** | Platform-level functionality and safety mechanisms | Standards, customer needs |
| **Feature Requirement** | Integration-level behaviour, independent of component decomposition | Stakeholder Requirements |
| **Component Requirement** | Component-specific implementation details | Feature Requirements |
| **Assumption of Use (AoU)** | Boundary conditions for using a software element (any level) | Safety analyses, architecture |

## Abstraction Boundaries (what each level must *not* specify)

Every level is **one step above detailed design**. Detailed design (concrete
error codes/types, message payload layouts, data structures, algorithms,
function signatures, timing constants, …) is **not** a requirement level and is
never demanded by a requirement review. When reviewing, judge completeness only
against the element's own level:

- **Stakeholder** — describes *what* the platform must offer and which safety
  mechanisms apply; does not name features, components, APIs, or mechanisms.
- **Feature** — describes integration-level, externally observable behaviour;
  does not prescribe component decomposition, internal interfaces, or
  implementation mechanisms (those belong to component requirements / design).
- **Component** — describes a component's externally testable behaviour at
  integration-test level; it is **still one level above detailed design**.
  Stating that the component "shall report an error", "shall reject the request"
  or "shall return a result" is complete — the exact error type/code, payload
  structure, or internal algorithm is fixed in detailed design and must **not**
  be required here.
- **AoU** — states a boundary condition the using element must satisfy; does not
  prescribe how it is met.

Do **not** lower a score or raise a finding/suggestion because a requirement
omits detail owned by the level(s) below it.
