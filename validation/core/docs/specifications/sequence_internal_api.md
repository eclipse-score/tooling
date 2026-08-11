<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Sequence Internal API Specification

## Purpose

This validator enforces consistency between sequence diagrams and Internal API
diagrams:

- **Sequence diagrams**
- **Internal API diagrams**

It checks Internal API method coverage with sequence plus Internal API inputs.
When a **Component diagram** is also provided, the validator uses it as optional
context to check sequence method names against the related shared interfaces of
the participating units.

## What is Validated

All comparisons are case-sensitive.

Only direct function calls (solid arrows, e.g. `unit_1 -> unit_2 : GetData()`)
are validated. Return/response interactions (dashed arrows, e.g.
`unit_2 --> unit_1 : Ack`) are not checked against the internal API and do not
contribute to interface coverage.

Method names are extracted by taking the text before the first `(` and
trimming whitespace, so a sequence call `GetData(d: Data*)` is compared
against an internal API method declared as `GetData()` using the same name
`GetData`. Sequence interactions that reduce to an empty method name after
this extraction are silently skipped.

Method-name consistency and consumer/provider roles consistency are checked only
when component context is available. Without component context, the validator
only checks Interface Coverage; Method-Name Consistency and Consumer/Provider
Role Consistency are skipped entirely.

### Method-Name Consistency

Every function used in a sequence interaction must be declared in the related
Internal API interface context.

For cross-unit calls, the method must be declared on a shared interface of the
participating units as defined in the component diagram. For self-calls, the
method must be declared on one of the available interfaces, where "available"
means any interface from either the component diagram or the internal API
diagram (not limited to interfaces bound to that unit).
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceInternalApiMethodNameConsistency`)*

This check is deferred (no error is raised by this validator) in two cases,
because another validator is responsible for reporting the root cause:

- Cross-unit calls between units that share no interface at all — reported by
  the `component_sequence` validator's Interface-Connection Consistency check.
- Calls involving a unit whose component-diagram interface reference is not
  declared in the internal API diagram — reported by the
  `component_internal_api` validator's Interface Declaration Consistency check.

```text
' component diagram
component "Unit 1" as unit_1 <<unit>>
component "Unit 2" as unit_2 <<unit>>
interface "IData" as IData
unit_1 -( IData
unit_2 )- IData
```

```
' sequence diagram
participant "Unit 1" as unit_1
participant "Unit 2" as unit_2
unit_1 -> unit_2 : GetData()

' internal_api diagram
interface "IData" as IData <<interface>> {
  {abstract} GetData(): Data*
}
```

### Consumer/Provider Role Consistency

When component context is available, cross-unit sequence calls must align with
consumer/provider roles derived from the component diagram for shared
interfaces.

The caller shall require and the callee shall provide at least one shared
interface on which the called method is declared. Self-calls are excluded from
this check.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceInternalApiConsumerProviderRoleConsistency`)*

```text
' component diagram
component "Unit 1" as unit_1 <<unit>>
component "Unit 2" as unit_2 <<unit>>
interface "IData" as IData
unit_1 -( IData
unit_2 )- IData
```

```text
' sequence diagram
participant "Unit 1" as unit_1
participant "Unit 2" as unit_2
unit_1 -> unit_2 : GetData()
```

```text
' internal_api diagram
interface "IData" as IData <<interface>> {
  {abstract} GetData(): Data*
}
```

### Interface Coverage

Every function declared in an Internal API interface must be called in at least
one sequence interaction. Self-calls count as valid usage.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceInternalApiInterfaceCoverage`)*

Coverage is computed globally: every method declared on every internal API
interface is checked against the full set of method names observed anywhere in
the sequence diagrams, regardless of which units are involved in the call or
how many times it occurs. Repeated calls between the same caller/callee for
the same method are only counted once.

```text
' internal_api diagram
interface "IData" as IData <<interface>> {
  {abstract} GetData(): Data*
  {abstract} SetData(d: Data*): void
}
```

```text
' sequence diagram
participant "Unit 1" as unit_1
participant "Unit 2" as unit_2
unit_1 -> unit_2 : GetData()
unit_1 -> unit_2 : SetData(d)
```

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Method not declared in related interface | Method-Name Consistency |
| Invalid consumer/provider roles | Consumer/Provider Role Consistency |
| Internal API interface function not exercised | Interface Coverage |

## Debug Output

The validator emits debug output containing:

- observed sequence calls (`caller -> callee : method`)
- unit interface targets derived from the component diagram, when component
  context is available
- Internal API interfaces available for sequence validation
