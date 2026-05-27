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

Define the intended consistency rules between component-diagram units, the
participants observed in sequence diagrams, and the internal interfaces used by
sequence-diagram function calls.

This validator checks whether:

- unit aliases from component diagrams match the set of used participants
  collected from sequence diagrams
- each function used in a sequence interaction is declared in an interface
  associated with the caller and callee unit and available in the related
  internal API diagram

## What is Validated

Component-diagram units and sequence-diagram participants must match. The validator checks:

- unit aliases from the component diagram vs. participant aliases used in sequence diagrams
- matching is case-sensitive
- interface connections in the static component diagram vs. function-call
  connections between the corresponding units in sequence diagrams
- units connected by function calls in sequence diagrams vs. corresponding
  interface connections in the static component diagram
- sequence function names used in interactions vs. method names declared in
  interfaces referenced by the participating units in the component diagram

## Failure Cases

### Missing sequence participant

Validation fails when a unit alias defined in the component diagram does not
appear among the participants used in the sequence diagrams.

### Missing sequence interaction for interface-connected units

Validation fails when units are connected through an interface in the static
component diagram, but no corresponding function-call interaction between those
units is specified in the sequence diagram.

### Unexpected sequence participant

Validation fails when a sequence diagram uses a participant alias that does not
correspond to any unit alias declared in the component diagram.

### Missing unit interface relation

Validation fails when the caller unit or callee unit has no related interface
available for function-call validation.

### Missing internal API interface

Validation fails when an interface referenced by the caller or callee unit
cannot be found in the corresponding internal API diagram.

### Method not found in related interfaces

Validation fails when a function used in a sequence interaction is not declared
in the related interfaces defined for the caller and callee units.

The validator reports an error when any of the following applies:

- the function is declared only in interfaces related to the caller unit
- the function is declared only in interfaces related to the callee unit
- the function is declared on both sides, but not in a shared matching
  interface
- the function is not declared in any related interface

## Debug Output

The validator emits debug output containing:

- expected unit aliases
- observed caller/callee participants
- observed sequence calls (`caller -> callee : method`)
- caller and callee interface targets derived from the component diagram
- interfaces checked in the internal API diagram

## Example

If the component diagram defines the unit aliases `unit_1` and `unit_2`, then
the sequence diagrams must use the same participant aliases. In addition, a
function call such as `unit_1 -> unit_2 : GetData(...)` must be backed by an
interface referenced by `unit_1` and `unit_2` in the component diagram, and
that interface must declare a `GetData` method in the internal API diagram.

```plantuml
' component diagram
package "Package A" as package_a <<SEooC>> {
  component "Component A" as component_a <<component>> {
    component "Unit 1" as unit_1 <<unit>>
    component "Unit 2" as unit_2 <<unit>>

    interface "InternalInterface" as InternalInterface
    unit_1 --( InternalInterface
    unit_2 )-- InternalInterface
  }
}

' sequence diagram
participant "Unit 1" as unit_1
participant "Unit 2" as unit_2

unit_1 -> unit_2 : GetData()
unit_2 --> unit_1 : return : Data*

' internal_api diagram
interface "InternalInterface" as InternalInterface <<interface>>{
  {abstract} GetData(): Data*
}
```
