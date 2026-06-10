<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

### Supported component relations

- Association (no direction):
  - `A -- B`
  - `A .. B`

- Dependency (directional):
  - `A --> B`
  - `B <-- A` (equivalent reverse-direction syntax)
  - `A ..> B`
  - `B <.. A` (equivalent reverse-direction syntax)

- Interface binding (component-left only):
  - Provided interface:
    - `Component )- Interface`
  - Required interface:
    - `Component -( Interface`

  Note: Only component-to-interface binding forms are supported.

### Unsupported interface binding forms

The following forms are rejected:

- Interface )- Component
- Interface -( Component

### Generic lollipop decorators

The following forms are resolved as plain associations and do not carry interface-binding semantics:
  - `Component --() Interface`
  - `Interface ()-- Component`

Note: Use canonical component-left forms such as `Component )- Interface` or `Component -( Interface` when you need interface binding behavior.

### Resolver constraints

When interface bindings are used:

- Exactly one endpoint must be an interface.
- Interface-to-interface bindings are not allowed.
- Interface-left decorator forms are rejected.
- Port role (`portin`/`portout`) must be consistent with decorator role.
