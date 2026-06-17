<!-- ----------------------------------------------------------------------------
  Copyright (c) 2025 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Design Context: Sample SEooC

Background material provided to the AI reviewer as read-only reference. It is
**not** graded; it only helps the reviewer interpret the architecture under
review.

## Scope

This is a Safety Element out of Context (SEooC). The component is developed
without a concrete vehicle-level item, so assumptions of use (AoU) stand in for
the missing system context.

## Components

- The static design (`static_design.puml`) describes the component structure and
  its public interfaces.
- The dynamic design (`dynamic_design.puml`) describes the runtime interaction
  between the component and its environment.
- The public API (`public_api.puml`) defines the interfaces exposed to
  integrators.

## Assumptions of Use

- Integrators are responsible for satisfying the documented assumptions of use
  before relying on the component's safety claims.
- The execution environment provides the resources declared in the static
  design.
