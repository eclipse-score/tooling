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

Safety Analysis
===============

.. note::
   A complete working example covering ``safety_analysis`` and ``dependability_analysis`` is
   available in
   `bazel/rules/rules_score/examples/seooc/safety_analysis/ <https://github.com/eclipse-score/tooling/tree/main/bazel/rules/rules_score/examples/seooc/safety_analysis>`_.

The ``dependability_analysis`` rule summarizes all the dependability analyses
(Safety / Security) for a dependable element. A single element may have
multiple dependability analyses.

Overview
--------

Why safety analysis?
~~~~~~~~~~~~~~~~~~~~~

A safety analysis shall support the process to systematically identify failures which could
violate safety goals or safety requirements. It shall also help to identify their root causes and design
appropriate countermeasures.

Safety analyses are typically performed with two complementary methods:

- **FMEA (Failure Mode and Effects Analysis)** is an inductive, bottom-up method:
  it examines individual components mainly at their interfaces, how they can fail, and what effect those failures have on the overall system.
- **FTA (Fault Tree Analysis)** is a deductive, top-down method:
  depending on the context it starts from a safety goal / safety requirement / failure (mode) and traces back to the potential causes that could lead to it.

How safety analyses are used in this context
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

SEooCs are designed to be included in a system (e.g. platform). This means that from a system point
of view the SEooCs are the leaves of its FMEA. Thus failure modes of the SEooC can be
identified by applying the FMEA methodology (structured fault models) to its interface (aka public API).

In a second step the root causes of the identified failure modes need to be analyzed within the SEooC. This can be achieved by performing a Fault Tree Analysis (FTA). Each identified root cause then needs to be treated with appropriate safety measures to guarantee that a safety-related failure mode cannot occur in the first place.

So the single steps are:

1. **Identify failure modes** — apply structured fault models (see
   `Fault models`_) to each public interface to derive what can cause a
   violation of an overarching safety goal.
2. **Analyze effects and causes** — decompose the identified failure modes
   into their root causes using a Fault Tree Analysis (FTA).
3. **Define countermeasures** — for every root cause specify a
   ``ControlMeasure`` (or ``PreventiveMeasure`` / ``Mitigation`` / ``AoU``) and
   trace it back through the FTA to the failure mode.
4. **Wire and validate** — bundle the resulting artifacts in Bazel and let the
   traceability check verify that they are consistently linked.

Each step is described in detail in `Performing the Analysis`_.

Fault models
~~~~~~~~~~~~~

The failure modes to consider are defined by the SCORE process:

    `FMEA Fault Models — Process Description <https://eclipse-score.github.io/process_description/main/process_areas/safety_analysis/guidance/fault_models_guideline.html#id1>`_

The fault models cover three categories: **messages** (send/receive behaviour),
**time constraints** (too early / too late), and **execution** (wrong result,
loss, delay, corruption, non-determinism). The ``Guideword`` enum in the
``ScoreReq`` model maps each category to a structured label used in the
``FailureMode`` records.

Artifacts and traceability
~~~~~~~~~~~~~~~~~~~~~~~~~~

As mentioned above, the safety analysis method used by ``dependability_analysis`` is a combination of both an FMEA and a FTA.
Each ``safety_analysis`` target bundles four types of artifacts that must be linked together:

.. list-table::
   :header-rows: 1

   * - Artifact
     - Format
     - What it represents
     - Created in
   * - **Public API Interfaces**
     - PlantUML (from ``architectural_design.public_api``)
     - Interfaces where failures can manifest; referenced by ``FailureMode.interface``
     - :doc:`architectural_design`
   * - **Failure Modes**
     - TRLC (``.trlc``)
     - Effects identified in the FMEA: what can go wrong and its impact
     - Step 1
   * - **FTA Diagrams**
     - PlantUML (``.puml``)
     - Fault Tree Analysis: structural decomposition of each failure mode into root causes
     - Step 2
   * - **Control Measures**
     - TRLC (``.trlc``)
     - Countermeasures that address the root causes identified in the FTA
     - Step 3

The artifacts are linked purely by naming conventions — no separate linking
step is needed:

- ``FailureMode.interface`` references an element of the ``public_api`` of the
  ``architectural_design`` target. This connects the architectural view to the
  safety analysis.
- The **alias** of a ``$TopEvent`` in an FTA diagram is the **TRLC
  fully-qualified record name** (``Package.RecordName``) of the
  ``FailureMode`` it decomposes.
- The **alias** of a ``$BasicEvent`` in an FTA diagram is the TRLC
  fully-qualified record name of the measure that addresses this root cause.

The traceability check verifies these links automatically (see
`Traceability Validation`_).

Performing the Analysis
-----------------------

The Bazel rule and traceability check only verify that the artifacts are
*linked* and *traced* — they cannot tell you whether the analysis is *complete or correct*.
Identifying failure modes, reasoning about causes, and choosing countermeasures
is a safety-engineering activity.

The following steps walk through this activity. Each step first explains the
safety reasoning and then shows how its result is recorded as an artifact. All
snippets are taken from the SEooC example in
``bazel/rules/rules_score/examples/seooc``.

Step 1 — Identify failure modes per interface
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Go through the ``public_api`` **method by method**. For each method, walk the
applicable fault models and ask *"can this occur, and would it violate a safety
goal or safety requirement?"*

The ``Guideword`` enum labels the fault-model category on each ``FailureMode``:

.. list-table::
   :header-rows: 1
   :widths: 22 33 45

   * - Fault-model category
     - Example fault models
     - ``Guideword`` labels
   * - **Message** (send/receive)
     - not sent / not received, corrupted, lost, unintended
     - ``LossOfFunction``, ``PartialFunction``, ``Corrupted``,
       ``UnintendedFunction``, ``Wrong``
   * - **Timing / duration constraint**
     - too late / too early, boundary violated
     - ``TooEarly``, ``TooLate``, ``DelayedFunction``
   * - **Execution**
     - wrong result, loss of execution, arbitrary/incomplete
     - ``Wrong``, ``LossOfFunction``, ``ExceedingFunction``, ``ArbitraryExecution``

Recording failure modes (TRLC)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Each identified failure mode is recorded as a ``ScoreReq.FailureMode`` record
in a ``.trlc`` file (here ``sample_safety_analysis_failure_modes.trlc``):

.. code-block:: text

    package SampleLibrary

    import ScoreReq

    ScoreReq.FailureMode SampleFailureMode{
        guidewords = [ScoreReq.Guideword.LossOfFunction]
        description = "SampleFailureMode takes over the world"
        failureeffect = "The world as we know it will end"
        version = 1
        safety = ScoreReq.Asil.B
        interface = "safety_software_seooc_example.SampleLibraryAPI.GetNumber"
    }

.. list-table::
   :header-rows: 1
   :widths: 25 75

   * - Attribute
     - Meaning
   * - ``guidewords``
     - One or more ``Guideword`` values classifying the failure (see table above).
   * - ``description``
     - What goes wrong.
   * - ``failureeffect``
     - Consequence for the caller / system (see Step 2).
   * - ``interface``
     - Fully-qualified name of the affected ``public_api`` element
       (``<namespace>.<Interface>.<Method>``). Links the failure mode to the
       architecture.
   * - ``safety``
     - ASIL of the safety goal the failure mode can violate (see *ASIL
       rationale* in Step 3).
   * - ``version``
     - Monotonically increasing counter; increment on every content change.
   * - ``rationale`` (optional)
     - Why this failure mode is considered relevant.

The record's fully-qualified name, ``SampleLibrary.SampleFailureMode``
(package + record name), becomes the ``$TopEvent`` alias of its fault tree in
Step 2.

Step 2 — Analyze the effect, then decompose to root causes
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

- **Effect** — describe the consequence **from the caller / system
  perspective**, in worst-case terms, relative to the safety goal. It is
  recorded in the ``failureeffect`` attribute of the ``FailureMode``.
- **Causes** — build a Fault Tree (FTA) top-down from the failure mode to its
  **root causes**:

  - Use an **OR gate** when *any single* child cause is sufficient to produce the
    parent — this is the default for independent causes.
  - Use an **AND gate** only when *all* children must occur together (e.g. a fault
    plus the failure of a safety mechanism) — this is what justifies a lower
    residual risk.
  - Decompose until each leaf (``$BasicEvent``) is an **actionable root cause** you
    can place a measure on — not a vague restatement of the failure.

Modeling the fault tree (PlantUML)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Each failure mode gets its own FTA diagram. A dedicated PlantUML metamodel
(`fta_metamodel.puml <https://github.com/eclipse-score/tooling/blob/main/plantuml/fta_metamodel.puml>`_,
located at ``plantuml/fta_metamodel.puml`` in the score-tooling repository)
provides the graphical elements as procedures; no standard PlantUML shapes are
needed. Every FTA ``.puml`` file must begin with ``!include fta_metamodel.puml``
so that the procedure definitions are available.

.. list-table::
   :header-rows: 1
   :widths: 35 65

   * - Procedure
     - Description
   * - ``$TopEvent(name, alias)``
     - The failure mode at the root of the tree. ``alias`` must equal the
       fully-qualified TRLC name of the corresponding ``FailureMode`` record
       (e.g. ``SampleLibrary.SampleFailureMode``).
   * - ``$IntermediateEvent(name, alias, connection)``
     - An intermediate cause that is decomposed further. ``connection`` is the
       alias of the parent node this event feeds into.
   * - ``$BasicEvent(name, alias, connection)``
     - A root cause (leaf node). ``alias`` must equal the fully-qualified TRLC
       name of the measure record that addresses it (Step 3). ``connection`` is
       the alias of the parent gate.
   * - ``$AndGate(alias, connection)``
     - AND gate: all children must occur for the parent to occur.
       ``connection`` is the alias of the parent node.
   * - ``$OrGate(alias, connection)``
     - OR gate: any single child is sufficient for the parent to occur.
       ``connection`` is the alias of the parent node.
   * - ``$TransferInGate(alias, connection)``
     - Transfer-in gate linking to another FTA sub-tree. ``alias`` is the
       fully-qualified TRLC name of that sub-tree's ``$TopEvent``;
       ``connection`` is the alias of the parent node.

Each element points to its **parent** via the ``connection`` parameter — the
arrow goes *from* the element *up* to the parent. Declare the tree from the top
down:

1. Declare the ``$TopEvent`` first (no ``connection`` parameter — it is the root).
2. Declare the gate(s) with ``connection`` set to the ``$TopEvent`` alias.
3. Declare ``$IntermediateEvent`` / ``$BasicEvent`` nodes with ``connection``
   set to the enclosing gate's alias. An ``$IntermediateEvent`` is decomposed
   further by gates whose ``connection`` is its alias.

::

    $TopEvent(alias="Pkg.FailureMode")  ← root, no connection
        └── $OrGate(alias="OG1", connection="Pkg.FailureMode")
                ├── $BasicEvent(alias="Pkg.MeasureA", connection="OG1")
                └── $BasicEvent(alias="Pkg.MeasureB", connection="OG1")

Gate and intermediate-event aliases (e.g. ``OG1``) are only local identifiers
within the diagram; they do not refer to TRLC records.

For example, the fault tree for ``SampleLibrary.SampleFailureMode`` (the
``FailureMode`` from Step 1) decomposes as follows:

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

``SampleLibrary.JustBadLuck`` alone is sufficient to cause the failure mode (OR
gate), whereas ``SampleLibrary.NoMoreCookies`` and
``SampleLibrary.NoMoreCoffee`` must occur together (AND gate) to cause the
intermediate event. Each of these basic events needs a measure in Step 3.

Step 3 — Choose a countermeasure for every root cause
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Every ``$BasicEvent`` needs exactly one measure record. Pick the type by *when* it
acts:

.. list-table::
   :header-rows: 1
   :widths: 24 46 30

   * - Type
     - Use when the measure…
     - Acts
   * - ``PreventiveMeasure``
     - removes the cause so the fault cannot occur.
     - before
   * - ``ControlMeasure``
     - detects and handles the fault at runtime (plausibility check, monitor).
     - during
   * - ``Mitigation``
     - reduces severity/probability after the fault has occurred.
     - after
   * - ``AoU`` (Assumption of Use)
     - can only be guaranteed by the **integrator/caller**, not inside the SEooC.
     - at integration

Recording measures (TRLC)
^^^^^^^^^^^^^^^^^^^^^^^^^

Each measure is a TRLC record whose fully-qualified name (package + record
name) equals the alias of the ``$BasicEvent`` it addresses. For example, the
following ``ControlMeasure`` records (from
``sample_safety_analysis_control_measures.trlc``) address the ``JustBadLuck``,
``NoMoreCookies``, and ``NoMoreCoffee`` basic events from the FTA diagram in
Step 2:

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

The ``ScoreReq`` model also defines ``PreventiveMeasure`` and ``Mitigation``
record types. The record type name changes, but the FTA alias convention
(package + record name matching the ``$BasicEvent`` alias) is identical.

An ``AoU`` is how you *push an obligation outward* when the SEooC cannot close a
root cause itself — it must be forwarded to the integrating project (see
:doc:`assumptions_of_use`).

**ASIL rationale:** the ``safety`` level on a ``FailureMode`` follows the safety
goal it can violate; a ``ControlMeasure`` that an ASIL argument relies on inherits
that level. Record *why* a measure is sufficient — an AND-gate decomposition or a
diagnostic coverage claim — rather than only *that* it exists.

Step 4 — Wire the analysis into Bazel
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The artifacts from Steps 1–3 are bundled in a ``safety_analysis`` target. Its
``failuremodes``, ``controlmeasures`` and ``root_causes`` files must live in the
**same package** as the ``safety_analysis`` target itself (Bazel does not allow
referencing another package's raw source files without ``exports_files``).
``arch_design`` points to the ``architectural_design`` target whose
``public_api`` is referenced by ``FailureMode.interface``:

.. code-block:: starlark
   :caption: bazel/rules/rules_score/examples/seooc/safety_analysis/BUILD

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "safety_analysis",
   )

   filegroup(
       name = "sample_fta",
       srcs = [
           "sample_fta.puml",
           "sample_fta2.puml",
       ],
       visibility = ["//visibility:public"],
   )

   safety_analysis(
       name = "sample_safety_analysis",
       arch_design = "//design:sample_seooc_design",
       controlmeasures = ["sample_safety_analysis_control_measures.trlc"],
       failuremodes = ["sample_safety_analysis_failure_modes.trlc"],
       root_causes = [":sample_fta"],
       visibility = ["//visibility:public"],
   )

The ``dependability_analysis`` target of the dependable element then
references one or more ``safety_analysis`` targets by label:

.. code-block:: starlark
   :caption: bazel/rules/rules_score/examples/seooc/BUILD

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "dependability_analysis",
   )

   dependability_analysis(
       name = "sample_dependability_analysis",
       arch_design = "//design:sample_seooc_design",
       safety_analysis = ["//safety_analysis:sample_safety_analysis"],
   )

**Generated targets:** ``<name>`` — ``bazel build`` produces the documentation
and traceability report; ``bazel test`` validates the full chain (see
`Traceability Validation`_).

For the complete attribute reference, see
:ref:`safety_analysis <rule-safety-analysis>` and
:ref:`dependability_analysis <rule-dependability-analysis>` in the rule index.

Traceability Validation
------------------------

Running ``bazel test`` on the ``dependability_analysis`` target (e.g.
``bazel test //:sample_dependability_analysis`` in the SEooC example) executes
a traceability check that validates the complete chain:

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
