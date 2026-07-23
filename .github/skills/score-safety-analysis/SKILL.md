<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

---
name: score-safety-analysis
description: "Step-by-step workflow for creating or extending a FMEA-based safety analysis in TRLC format for S-CORE software components. Use when asked to: add failure modes, create FTA diagrams, add control measures, or validate the safety analysis traceability chain. Covers clustering, FailureMode records, FTA PlantUML files, ControlMeasure records, BUILD wiring, and trlc validation."
argument-hint: "interface or component name to analyse"
---

# S-CORE Safety Analysis — TRLC Workflow

## When to Use

- Adding new failure modes to an existing safety analysis
- Creating FTA diagrams for root-cause decomposition
- Defining ControlMeasure / AoU records for identified root causes
- Validating the full traceability chain before a review of a safety analysis

## Key Files and Locations

```
score/<component>/dependability/
├── safety_analysis/
│   ├── failure_modes.trlc      # FailureMode records (one per unique root-cause cluster)
│   ├── control_measures.trlc   # ControlMeasure / PreventiveMeasure / AoU records
│   ├── fta_<failure_mode>.puml # One FTA diagram per FailureMode
│   └── BUILD                   # fmea() rule — must list all .puml in fta_files filegroup
├── assumed_system/
│   └── aous.trlc               # AoU records (caller obligations)
└── requirements/
    └── component_requirements.trlc
```

The canonical RSL (type definitions) lives in:
`eclipse-score-tooling/bazel/rules/rules_score/trlc/config/score_requirements_model.rsl`

## Step 1 — Write FailureMode Records (`failure_modes.trlc`)

Package: match the existing package declaration in the folder structure

```trlc
package <Pkg>
import ScoreReq

ScoreReq.FailureMode <RecordName> {
    guideword      = ScoreReq.GuideWord.<Word>
    description    = "statement describing the expected behaviour"
    failureeffect  = "What goes wrong for the caller / system"
    potentialcause = "Root cause(s) from the FMEA"
    interface      = "<Interface>.<Method>[, <Interface>.<Method>]"
    version        = 1
    safety         = ScoreReq.Asil.B
}
```

**GuideWord enum values:** `LossOfFunction`, `PartialFunction`, `Corrupted`, `UnintendedFunction`, `TooEarly`, `TooLate`, `Wrong`, `DelayedFunction`, `ExceedingFunction`, `ArbitraryExecution`

**Rules:**
- One record per FailureMode, not per interface method.
- Same root cause spanning multiple guide words → separate records (trlc only allows one `guideword` per record).
- `interface` may list multiple methods as a comma-separated string when root cause is shared.

## Step 2 — Create FTA Diagrams (`fta_<snake_name>.puml`)

One `.puml` file per `FailureMode` record. The `$TopEvent` alias **must** equal the fully-qualified TRLC record name (`<Package>.<RecordName>`).

```plantuml
@startuml

!include fta_metamodel.puml

$TopEvent("<Human-readable top event label>", "<Pkg>.<FailureModeRecord>")

$OrGate("OG1", "<Pkg>.<FailureModeRecord>")

$BasicEvent("<Root cause label>", "<Pkg>.<ControlMeasureRecord>", "OG1")
$BasicEvent("<Root cause label 2>", "<Pkg>.<ControlMeasureRecord2>", "OG1")

@enduml
```

**Procedures reference:**

| Procedure | Purpose | `connection` points to |
|-----------|---------|------------------------|
| `$TopEvent(name, alias)` | Top failure mode | — (root, no connection) |
| `$OrGate(alias, connection)` | Any child sufficient | parent alias |
| `$AndGate(alias, connection)` | All children required | parent alias |
| `$BasicEvent(name, alias, connection)` | Root cause / leaf | enclosing gate alias |
| `$IntermediateEvent(name, alias, connection)` | Intermediate cause | parent gate alias |
| `$TransferInGate(name, alias, connection)` | Link to sub-tree | parent alias |

**Rules:**
- `$BasicEvent` alias = `<Package>.<ControlMeasureRecordName>` — this IS the traceability link.
- Build bottom-up in the file: `$TopEvent` first, then gates, then `$BasicEvent` leaves.
- The same `ControlMeasure` alias may appear in multiple FTAs (shared root cause).
- `$OrGate` is the default for independent root causes; use `$AndGate` only when all causes must co-occur.

## Step 3 — Write ControlMeasure Records (`control_measures.trlc`)

For every `$BasicEvent` alias in every FTA, define a matching record:

```trlc
ScoreReq.ControlMeasure <RecordName> {
    safety      = ScoreReq.Asil.B
    description = "Normative measure text"
    version     = 1
}
```

Other available types (same pattern, different semantics):
- `ScoreReq.PreventiveMeasure` — prevents the failure from occurring
- `ScoreReq.Mitigation` — reduces severity/probability after occurrence
- `ScoreReq.AoU` — assumption the caller must satisfy; add `mitigates = "<RecordName>"` field

**Rule:** `<Package>.<RecordName>` in TRLC must match the `$BasicEvent` alias verbatim.

## Step 4 — Update BUILD

Add every new `.puml` to the `fta_files` filegroup **and** keep the list alphabetically sorted:

```python
filegroup(
    name = "fta_files",
    srcs = [
        "fta_api_called_before_lifecycle_ready.puml",
        "fta_client_connection_failed.puml",
        # ... one entry per FTA file, alphabetical
    ],
    visibility = ["//score/<component>/dependability:__pkg__"],
)
```

## Step 5 — Validate

**Pass criteria:** zero errors in `failure_modes.trlc`, `control_measures.trlc`, `aous.trlc`.
Pre-existing RSL union-type errors (`expected identifier, encountered '['`) are a known trlc v2 / RSL version mismatch — ignore if they appear only in the tooling RSL, not in component files.

**Traceability chain that must be complete:**

```
FailureMode.interface  →  public_api interface name
FailureMode record     →  $TopEvent alias
$BasicEvent alias      →  ControlMeasure / AoU record name
```

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| `$BasicEvent` alias does not match any TRLC record | Ensure `<Pkg>.<RecordName>` is spelled identically in both places |
| New `.puml` not in BUILD `fta_files` | Add the file path to the `srcs` list |
| AoU added to `control_measures.trlc` | AoUs belong in `aous.trlc`; both extend `Measure` so the FTA alias still resolves |
| Wrong RSL used for trlc validation   | Always pass the tooling RSL as the first directory argument |
