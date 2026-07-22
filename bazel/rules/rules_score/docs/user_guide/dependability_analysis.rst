..
   # *******************************************************************************
   # Copyright (c) 2026 Contributors to the Eclipse Foundation
   #
   # See the NOTICE file(s) distributed with this work for additional
   # information regarding copyright ownership.
   #
   # This program and the accompanying materials are made available under the
   # terms of the Apache License Version 2.0 which is available at
   # https://www.apache.org/licenses/LICENSE-2.0
   #
   # SPDX-License-Identifier: Apache-2.0
   # *******************************************************************************

Dependability Analysis
=======================

.. note::
   A complete working example covering ``fmea`` and ``dependability_analysis`` is
   available in
   `bazel/rules/rules_score/examples/seooc/safety_analysis/ <https://github.com/eclipse-score/tooling/tree/main/bazel/rules/rules_score/examples/seooc/safety_analysis>`_.

The ``dependability_analysis`` rule summarizes all the dependability analyses
(Safety / Security) for a dependable element. A single element may have
multiple dependability analyses.

Overview
--------

Why safety analysis?
~~~~~~~~~~~~~~~~~~~~~

Safety analysis is required to systematically identify failures that could
violate safety goals and to demonstrate that appropriate countermeasures are
in place. In ISO 26262 terms it provides the evidence that residual risk is
acceptable.

How FMEA works
~~~~~~~~~~~~~~~

A Failure Mode and Effects Analysis (FMEA) follows three steps for each public
interface of the software module:

1. **Identify failure modes** — apply structured fault models (see below) to
   each public interface to derive what can cause a violation of a
   overarching safety goal.
2. **Analyse effects and causes** — document the effect on the system and
   decompose to root causes using a Fault Tree Analysis (FTA).
3. **Define countermeasures** — for every root cause specify a
   ``ControlMeasure`` (or ``PreventiveMeasure`` / ``Mitigation``) and trace it
   back through the FTA to the failure mode.

Fault models
~~~~~~~~~~~~~

The failure modes to consider are defined by the SCORE process:

    `FMEA Fault Models — Process Description <https://eclipse-score.github.io/process_description/main/process_areas/safety_analysis/guidance/fault_models_guideline.html#id1>`_

The fault models cover three categories: **messages** (send/receive behaviour),
**time constraints** (too early / too late), and **execution** (wrong result,
loss, delay, corruption, non-determinism). The ``Guideword`` enum in the
``ScoreReq`` model maps each category to a structured label used in the
``FailureMode`` records.

The description below covers the FMEA-based **safety** analysis for a
software module.

Bazel Rule ``dependability_analysis``
----------------------------------------

.. code-block:: starlark

    load("@score_tooling//bazel/rules/rules_score:rules_score.bzl",
         "dependability_analysis")

    dependability_analysis(
        name        = "my_da",
        arch_design = ":my_arch",
        fmea        = [":my_fmea"],
    )

**Generated targets:** ``<name>`` — build produces the documentation and
traceability report; ``bazel test`` validates the full chain.

FMEA
----

The Failure Mode and Effects Analysis (FMEA) is the core safety analysis
method used by ``dependability_analysis``. Each ``fmea`` target bundles four
types of artifacts that must be linked together:

.. list-table::
   :header-rows: 1

   * - Artifact
     - Format
     - What it represents
   * - **Public API Interfaces**
     - PlantUML (from ``architectural_design.public_api``)
     - Interfaces where failures can manifest; referenced by ``FailureMode.interface``
   * - **Failure Modes**
     - TRLC (``.trlc``)
     - Effects identified in the FMEA: what can go wrong and its impact
   * - **FTA Diagrams**
     - PlantUML (``.puml``)
     - Fault Tree Analysis: structural decomposition of each failure mode into root causes
   * - **Control Measures**
     - TRLC (``.trlc``)
     - Countermeasures that address the root causes identified in the FTA

The public API connects the architectural view to the safety analysis:
``FailureMode.interface`` references an interface name defined in the
``public_api`` of the ``architectural_design`` target.

The FTA artifacts are linked by a shared naming convention: the **TRLC
fully-qualified record name** (package + record name) must match the
**alias** used in the FTA PlantUML diagram. This is how traceability is
established automatically in the report.

Failure Modes (TRLC)
~~~~~~~~~~~~~~~~~~~~~

A failure mode is a ``FailureMode`` record in the ``ScoreReq`` model. The
example below is taken from ``examples/seooc/safety_analysis``:

.. code-block:: text

    package SampleLibrary

    import ScoreReq

    ScoreReq.FailureMode SampleFailureMode{
        guidewords = [ScoreReq.Guideword.LossOfFunction]
        description = "SampleFailureMode takes over the world"
        failureeffect = "The world as we know it will end"
        version = 1
        safety = ScoreReq.Asil.B
        interface = "SampleLibraryAPI.GetNumber"
    }

The TRLC fully-qualified name of this record is
**``SampleLibrary.SampleFailureMode``**. This name is used as the
``$TopEvent`` alias in the FTA diagram.

FTA Diagrams (PlantUML)
~~~~~~~~~~~~~~~~~~~~~~~~

Each failure mode gets a Fault Tree Analysis diagram. A dedicated PlantUML
metamodel
(`fta_metamodel.puml <https://github.com/eclipse-score/tooling/blob/main/plantuml/fta_metamodel.puml>`_)
provides the graphical elements — it is located at
``plantuml/fta_metamodel.puml`` in the score-tooling repository. Your diagram
uses procedure calls from that metamodel; no standard PlantUML shapes are
needed.

Every ``.puml`` FTA file must begin with ``!include fta_metamodel.puml`` so
that the procedure definitions are available.

Available procedures
^^^^^^^^^^^^^^^^^^^^^

.. list-table::
   :header-rows: 1

   * - Procedure
     - Description
   * - ``$TopEvent(name, alias)``
     - The top-level failure mode. ``alias`` must equal the fully-qualified TRLC name of the corresponding ``FailureMode`` record (e.g. ``SampleLibrary.SampleFailureMode``)
   * - ``$IntermediateEvent(name, alias, connection)``
     - An intermediate cause. ``connection`` is the **alias of the parent** node this event feeds into
   * - ``$BasicEvent(name, alias, connection)``
     - A root cause (leaf node). ``alias`` must equal the fully-qualified TRLC name of the corresponding ``ControlMeasure`` record. ``connection`` is the alias of the parent gate
   * - ``$AndGate(alias, connection)``
     - AND gate. All children must occur for the parent to trigger. ``connection`` is the alias of the parent node
   * - ``$OrGate(alias, connection)``
     - OR gate. Any single child is sufficient to trigger the parent. ``connection`` is the alias of the parent node
   * - ``$TransferInGate(name, alias, connection)``
     - Transfer-in gate linking to another FTA sub-tree

Linking procedures together
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Each element points to its **parent** via the ``connection`` parameter — the
arrow goes *from* the element *up* to the parent. Build the tree bottom-up:

1. Declare the ``$TopEvent`` first (no ``connection`` parameter — it is the root).
2. Declare gate(s) with ``connection`` set to the ``$TopEvent`` alias.
3. Declare ``$BasicEvent`` / ``$IntermediateEvent`` nodes with ``connection``
   set to the enclosing gate's alias.

::

    $TopEvent  ← root, no connection
        └── $OrGate(alias="OG_1", connection="TopEvent.alias")
                ├── $BasicEvent(alias="CM_A", connection="OG_1")
                └── $BasicEvent(alias="CM_B", connection="OG_1")

The ``$BasicEvent`` **alias IS the fully-qualified TRLC name**
(``Package.RecordName``) of the corresponding ``ControlMeasure`` record. No
separate linking step is needed — the naming convention is the link.

Example FTA diagram
^^^^^^^^^^^^^^^^^^^^

.. uml:: ../_assets/SeoocExample_FTA.puml
   :align: center
   :alt: Example FTA diagram

.. code-block:: text

    @startuml SeoocExample_FTA
    !include fta_metamodel.puml

    $TopEvent("SampleFailureMode takes over the world", "SampleLibrary.SampleFailureMode")

    $OrGate("OG1", "SampleLibrary.SampleFailureMode")

    $IntermediateEvent("SampleFailureMode is Angry", "IEF", "OG1")
    $BasicEvent("Just bad luck", "SampleLibrary.JustBadLuck", "OG1")

    $AndGate("AG2", "IEF")
    $BasicEvent("No More Cookies", "SampleLibrary.NoMoreCookies", "AG2")
    $BasicEvent("No More Coffee", "SampleLibrary.NoMoreCoffee", "AG2")

    @enduml

Control Measures (TRLC)
~~~~~~~~~~~~~~~~~~~~~~~~

For each ``$BasicEvent`` in your FTA diagram, define a ``ControlMeasure``
record whose fully-qualified name matches the event alias:

.. code-block:: text

    package SampleLibrary

    import ScoreReq

    ScoreReq.ControlMeasure JustBadLuck{
        safety = ScoreReq.Asil.B
        description = "Sometimes, the dark side wins. We shall be prepared for that."
        version = 1
    }

    ScoreReq.ControlMeasure NoMoreCookies{
        safety = ScoreReq.Asil.B
        description = "We shall only order family size cookie jars"
        version = 1
    }

    ScoreReq.ControlMeasure NoMoreCoffee{
        safety = ScoreReq.Asil.B
        description = "We shall keep a coffee reserve for emergencies"
        version = 1
    }

The alias ``SampleLibrary.JustBadLuck`` in the FTA diagram matches the TRLC
record ``JustBadLuck`` in package ``SampleLibrary`` — and likewise for
``NoMoreCookies``/``NoMoreCoffee``. This is how the traceability link is
established.

Other measure types
^^^^^^^^^^^^^^^^^^^^

The SCORE requirements model also defines ``PreventiveMeasure`` and
``Mitigation``, both extending the same abstract ``Measure`` base type as
``ControlMeasure``. Their Bazel and TRLC usage follows the same pattern; the
record type name changes but the FTA alias convention (package + record name
matching the ``$BasicEvent`` alias) is identical.

``fmea`` — Bazel Rule
~~~~~~~~~~~~~~~~~~~~~~

For the complete ``fmea`` attribute reference, see :ref:`fmea <rule-fmea>` in
the rule index.

Traceability Validation
------------------------

Running ``bazel test //my/package:my_da`` executes a traceability check that
validates the complete chain:

::

          public_api interface ← FailureMode.interface
                                            |
                                        $TopEvent
                                            |
                                     AND / OR gate(s)
                                            |
                                       $BasicEvent
                                            |
                                      ControlMeasure

The check fails if:

- A ``$TopEvent`` alias does not match any ``FailureMode`` record name
- A ``$BasicEvent`` alias does not match any ``ControlMeasure`` record name
- A ``FailureMode`` or ``ControlMeasure`` is defined but not referenced in any FTA diagram

Fixing a traceability error means ensuring the naming convention is followed
precisely: the fully-qualified TRLC name (package + record name, e.g.
``SampleLibrary.JustBadLuck``) must be used verbatim as the alias in the FTA diagram.

Example
-------

The ``fmea`` rule's ``failuremodes``/``controlmeasures``/``root_causes``
files must live in the **same package** as the ``fmea`` target itself (Bazel
does not allow referencing another package's raw source files without
``exports_files``). The parent ``dependability_analysis`` target then
references the ``fmea`` target by label:

.. code-block:: starlark
   :caption: bazel/rules/rules_score/examples/seooc/safety_analysis/BUILD

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "fmea",
   )

   filegroup(
       name = "sample_fta",
       srcs = [
           "sample_fta.puml",
           "sample_fta2.puml",
       ],
       visibility = ["//visibility:public"],
   )

   fmea(
       name = "sample_fmea",
       arch_design = "//design:sample_seooc_design",
       controlmeasures = ["sample_fmea_control_measures.trlc"],
       failuremodes = ["sample_fmea_failure_modes.trlc"],
       root_causes = [":sample_fta"],
       visibility = ["//visibility:public"],
   )

.. code-block:: starlark
   :caption: bazel/rules/rules_score/examples/seooc/BUILD

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "dependability_analysis",
   )

   dependability_analysis(
       name        = "sample_dependability_analysis",
       arch_design = "//design:sample_seooc_design",
       fmea        = ["//safety_analysis:sample_fmea"],
   )
