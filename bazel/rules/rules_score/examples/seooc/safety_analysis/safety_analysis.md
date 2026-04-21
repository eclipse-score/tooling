<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Safety Analysis

This document shows the workflow and the interrelation on how to document an FMEA in TRLC / PlantUml / Lobster.

## Overview

The current proposal on how the Safety Analysis according ISO 26262 Pt9 shall be implemented is as described in the Model:

![Safety Analysis](assets/safety_analysis.svg)

## Implementation

For the implementation of the Safety Analysis a Mix of TRLC and Plantuml is applied. The Verification itself is performed in lobster at the end.

For the Definition and Verification of the Safetyanalysis itself two Bazel rules exist:

```starlark
load("//bazel/rules/rules_score:rules_score.bzl", "dependability_analysis", "fmea")

fmea(
    name = "sample_fmea",
    controlmeasures = [<Link to trlc_requirements target>],
    failuremodes = [<Link to trlc_requirements target>],
    root_causes = [<Link to filegroup containing puml files>],
)

dependability_analysis(
    name = "sample_dependability_analysis",
    fmea = [":sample_fmea"],
)
```


### Failuremode

It starts with a Failuremode which was determined in an FMEA. This Failuremode is defined in a TRLC format:

```trlc
package SampleLibrary

import ScoreReq

ScoreReq.FailureMode SampleFailureMode{
    guideword = ScoreReq.GuideWord.LossOfFunction
    description = "SampleFailureMode takes over the world"
    failureeffect = "The world as we know it will end"
    version = 1
    safety = ScoreReq.Asil.B
    interface = "SampleLibraryAPI.GetNumber"
}
```

### Root Causes
As described in the metamodel the root causes (aka BasicEvents) shall be identified by performing an FTA on each Failuremode.

The FTA shall be modeled using Plantuml. Therefore a Metamodel was defined using plantuml procedures. It includes following entities:

Events:
- $TopEvent($name, $alias)
- $IntermediateEvent($name, $alias, $connection)
- $BasicEvent($name, $alias, $connection)

Gates:
- $AndGate($alias, $connection)
- $OrGate($alias, $connection)

The Matching between TRLC and Plantuml shall be performed using the TRLC ID. This means that also the TopEvent of the FTA shall use the TRLC ID of the Failuremode. For our case the Failuremode was defined in the package SampleLibrary. Therefore the Full ID of the TRLC node is:
SampleLibrary.SampleFailureMode

```plantuml
@startuml

!include fta_metamodel.puml

' Top level (skeleton)
$TopEvent("SampleFailureMode takes over the world", "SampleLibrary.SampleFailureMode")

' 2nd level gates and events
$OrGate("OG1", "SampleLibrary.SampleFailureMode")

$IntermediateEvent("SampleFailureMode is Angry", "IEF", "OG1")
$BasicEvent("Just bad luck", "SampleLibrary.JustBadLuck", "OG1")

' 3rd level cascades from AGF
$AndGate("AG2", "IEF")
$BasicEvent("No More Cookies", "SampleLibrary.NoMoreCookies", "AG2")
$BasicEvent("No More Coffee", "SampleLibrary.NoMoreCoffee", "AG2")
@enduml

```

![FTA Example](assets/fta_example.svg)

## Control Measures
For each BasicEvent a Control Measure shall be derived. This will be performed again in TRLC. The Mapping between PlantUML and TRLC will be done again based on the ID. So for our case we will define a Control Measure for NoMoreCookies:

```trlc
ScoreReq.ControlMeasure SampleAoU{
    safety = ScoreReq.Asil.B
    description = "We shall only order family size cookie jars"
    version = 1
    mitigates = "SampleLibrary.NoMoreCookies"
}

```

## Traceability Report

The `dependability_analysis` rule wrapping the `fmea` target is a Bazel test rule that runs a [lobster](https://github.com/bmw-software-engineering/lobster) traceability report via `lobster-ci-report`.

To run the report and check the traceability chain (FTA events → Failure Modes / Control Measures):

```bash
bazel test //bazel/rules/rules_score/examples/seooc:sample_dependability_analysis
```

## Tool Data Flow

The diagram below shows how the input files are processed by each tool and assembled into the final lobster traceability report.

![Tool Data Flow](assets/tool_data_flow.svg)

| Input | Tool | Output |
|---|---|---|
| `public_api.puml` | `puml_parser --fbs-output-dir --lobster-output-dir` | `architecture.lobster` — Architecture interface items |
| `failuremodes.trlc` | `lobster-trlc` | `failuremodes.lobster` — Failure Mode requirements |
| `controlmeasures.trlc` | `lobster-trlc` | `controlmeasures.lobster` — Control Measure requirements |
| `fta.puml` | `safety_analysis_tools` | `root_causes.lobster` — FTA TopEvent / BasicEvent activities |
| all `.lobster` files | `lobster-ci-report` | `report.json` — traceability check result |
| `report.json` | `lobster-html-report` | `report.html` — human-readable HTML report |
