<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Component Sequence Specification

## Purpose

Define the intended consistency rules between component-diagram units and the
participants observed in sequence diagrams.

This validator checks whether unit aliases from component diagrams match the
set of used participants collected from sequence diagrams.

## What is Validated

Component-diagram units and sequence-diagram participants must match. The validator checks:

- unit aliases from the component diagram vs. participant aliases used in sequence diagrams
- matching is case-sensitive

## Failure Cases

### Missing sequence participant

Validation fails when a unit alias defined in the component diagram does not
appear among the participants used in the sequence diagrams.

### Unexpected sequence participant

Validation fails when a sequence diagram uses a participant alias that does not
correspond to any unit alias declared in the component diagram.

## Debug Output

The validator emits a debug view containing:

- expected unit aliases
- observed caller/callee participants

## Example

If the component diagram defines the unit aliases `unit_1` and `unit_2`, then
the sequence diagrams must use the same participant aliases.

```plantuml
' component diagram
package "Package A" as package_a <<SEooC>> {
  component "Component A" as component_a <<component>> {
    component "Unit 1" as unit_1 <<unit>>
    component "Unit 2" as unit_2 <<unit>>
  }
}

' sequence diagram
participant "Unit 1" as unit_1
participant "Unit 2" as unit_2

unit_1 -> unit_2 : SendSignal
unit_2 --> unit_1 : Ack
```
