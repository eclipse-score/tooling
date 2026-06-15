<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# SCORE Project — Requirements Specification Conventions

Project-specific guidelines for the Eclipse SCORE project. Apply in conjunction with the general inspection guidelines.

## Project Context

- **Safety standard:** ISO 26262 (functional safety for road vehicles)
- **Target platforms:** Linux, QNX (and potentially others defined at system level)

## Requirement Levels

| Level | Scope | Derived From |
|---|---|---|
| **Stakeholder Requirement** | Platform-level functionality and safety mechanisms | Standards, customer needs |
| **Feature Requirement (FeatReq)** | Integration-level behaviour; may constrain architectural design | Stakeholder Requirements |
| **Component Requirement** | Component-specific behaviour and internal design constraints | Feature Requirements |
| **Assumption of Use (AoU)** | Boundary conditions imposed on the user of a software element | Safety analyses, architecture |

### Feature Requirements — Scope Rules

Feature Requirements **may** contain:
- A named architectural element as the subject (e.g. "The message passing component shall…") when constraining an **architectural design decision**. This does not lower the requirement to Component level.
- **High-level platform boundary conditions** (e.g. "under QNX", "on Linux") when they define the operating context, not the coding implementation.
- **Architectural constraints on design patterns** (e.g. "shall not use singletons", "shall allow dependency injection") when these prohibit or mandate design approaches at architecture level, not at code level.
- A reference to the applicable safety standard without repeating it (ISO 26262 applies project-wide).

Feature Requirements **shall not** contain:
- Specific algorithms, data structures, or internal API signatures
- Code organisation or module structure
- Low-level implementation steps

## Requirement Types

| Type | Meaning | Typical Verification |
|---|---|---|
| **Functional** | Observable behaviour of the system or component | Unit / integration test |
| **Interface** | API, protocol, or communication specification | Test or inspection |
| **Non-Functional** | Quality attribute (performance, reliability, safety integrity) | Review / analysis / measurement |
| **Process** | Constraint on a development or operational process | Process review |
