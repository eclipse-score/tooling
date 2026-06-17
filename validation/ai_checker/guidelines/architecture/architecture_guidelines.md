<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Architecture Design Guidelines

General guidelines for reviewing architectural design diagrams (PlantUML). Each
artefact is the raw PlantUML source of a static (component / class) or dynamic
(sequence) diagram. Project-specific details (e.g. the available architecture
levels) are supplied separately by the project guidelines.

## Quality Criteria

A well-formed architectural diagram is:

- **Named** — every component, interface, participant, and relation has an explicit, meaningful name (no anonymous or placeholder names).
- **Interface-explicit** — components communicate through declared interfaces, not direct undocumented coupling.
- **Consistent** — element names match the corresponding requirements and other diagrams; no contradictions across static and dynamic views.
- **Traceable** — components and interfaces map to feature/component requirements they realize.
- **Layered** — dependencies flow in one direction; no cyclic dependencies between components.
- **Cohesive** — each component has a single, clear responsibility.
- **Complete** — every relation referenced in a sequence diagram exists in the corresponding component/class diagram.

### Avoid

- Unconnected or orphan elements (declared but never related).
- Direct access bypassing declared interfaces.
- Cyclic dependencies between components.
- Overloaded components with unrelated responsibilities.
- Relations without stereotypes/labels where the semantics are not obvious.
- Mismatched names between the architecture and the requirements it realizes.

## Static vs Dynamic Consistency
