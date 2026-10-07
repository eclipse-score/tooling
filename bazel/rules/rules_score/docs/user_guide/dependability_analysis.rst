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
================

.. note::
   A complete working example covering ``failure_modes``, ``fault_trees``, ``safety_analysis`` and ``dependability_analysis`` is
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

In a second step the root causes of the identified failure modes need to be analyzed within the SEooC. This
can be achieved by performing a Fault Tree Analysis (FTA). Each identified root cause then needs to be
treated with appropriate safety measures to guarantee that a safety-related failure mode cannot occur in
the first place.

So the single steps are:

1. **Identify failure modes** — apply structured fault models (see
   `Fault models`_) to each public interface to derive what can cause a
   violation of an overarching safety goal.
2. **Analyze effects and causes** — decompose the identified failure modes
   into their root causes using a Fault Tree Analysis (FTA).
3. **Define countermeasures** — for every root cause, close it with a
   ``Mitigation``, a ``CompReq``, or push the obligation outward with an
   ``AoU``, and trace it back through the FTA to the root cause.
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

As mentioned above, the safety analysis method used by ``dependability_analysis`` is a combination of
both an FMEA and a FTA. The ``failure_modes``, ``fault_trees`` and ``safety_analysis`` targets together bundle four types of
artifacts that must be linked together:

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
   * - **Safety Measures**
     - TRLC (``.trlc``)
     - Countermeasures that address the root causes identified in the FTA
     - Step 3

The artifacts are linked as follows:

- ``FailureMode.interface`` references an element of the ``public_api`` of the
  ``architectural_design`` target. This connects the architectural view to the
  safety analysis.
- Every ``$FailureMode`` node in an FTA diagram names the **TRLC
  fully-qualified record name(s)** (``Package.RecordName``) of the
  ``FailureMode`` record(s) it covers.
- Every ``$RootCause`` leaf in an FTA diagram gets a plain-identifier alias. A
  dedicated tool generates a ``RootCause`` stub for each such alias; the stub
  lists the ``FailureMode`` record(s) of its fault tree in ``failure_modes``. A
  ``Mitigation``/``CompReq``/``AoU`` then references the stub explicitly from
  its ``root_causes``/``derived_from`` field.

The figure shows all record links; arrows point from the referencing record to
the referenced one:

.. uml:: ../_assets/safety_analysis_linkage.puml
   :align: center
   :alt: Record linkage of the safety analysis
   :width: 100%

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
in a ``.trlc`` file (here ``sample_safety_analysis_failure_modes.trlc``). The
TRLC ``package`` name (``SampleLibrary`` below) is chosen freely by the author —
it is not derived from the Bazel target name and is not enforced by the rule
(only the *generated* ``<fta_package>`` package, see `Modeling the fault tree
(PlantUML)`_, is fixed):

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
(package + record name), is passed as a ``$FailureMode`` argument in Step 2,
linking it to its fault tree.

Step 2 — Analyse the effect, then decompose to causes
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

- **Effect** (``failureeffect``) — describe the consequence **from the caller /
  system perspective**, in worst-case terms, relative to the safety goal. "Returns
  a stale value that the controller uses to actuate" is a usable effect; "function
  returns wrong data" is not.
- **Causes** — build the Fault Tree (FTA) top-down from the failure mode to its
  **root causes**:

  - Use an **OR gate** when *any single* child cause is sufficient to produce the
    parent — this is the default for independent causes.
  - Use an **AND gate** only when *all* children must occur together (e.g. a fault
    plus the failure of a safety mechanism) — this is what justifies a lower
    residual risk.
  - Decompose until each leaf (``$RootCause``) is an **actionable root cause** you
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
   * - ``$FailureMode(name, fm1, fm2, ..., fm8)``
     - The failure mode(s) at the root of the tree. Each ``fm*`` is the
       fully-qualified TRLC name (``Package.RecordName``) of a
       ``FailureMode`` record this node covers — at least one (``fm1``) is
       required, up to 8 total. ``fm1`` doubles as this node's alias, the
       connection point every gate/root-cause ancestor attaches to.
   * - ``$IntermediateEvent(name, alias, connection)``
     - An intermediate cause that is decomposed further. ``connection`` is the
       alias of the parent node this event feeds into.
   * - ``$RootCause(name, alias, connection)``
     - A root cause (leaf node). ``alias`` is a plain TRLC identifier — it
       becomes the record name of a generated ``RootCause`` stub, referenced
       explicitly from a measure's ``root_causes``/``derived_from`` field in
       Step 3. ``connection`` is the alias of the parent gate.
   * - ``$AndGate(alias, connection)``
     - AND gate: all children must occur for the parent to occur.
       ``connection`` is the alias of the parent node.
   * - ``$OrGate(alias, connection)``
     - OR gate: any single child is sufficient for the parent to occur.
       ``connection`` is the alias of the parent node.
   * - ``$TransferInGate(name, alias, connection)``
     - Transfer-in gate linking to another FTA sub-tree. ``connection`` is the
       alias of the parent node.

Each element points to its **parent** via the ``connection`` parameter — the
arrow goes *from* the element *up* to the parent. Declare the tree from the top
down:

1. Declare the ``$FailureMode`` first (no ``connection`` parameter — it is the root).
2. Declare the gate(s) with ``connection`` set to the ``$FailureMode`` alias (``fm1``).
3. Declare ``$IntermediateEvent`` / ``$RootCause`` nodes with ``connection``
   set to the enclosing gate's alias. An ``$IntermediateEvent`` is decomposed
   further by gates whose ``connection`` is its alias.

::

    $FailureMode  ← root, no connection
        └── $OrGate(alias="OG_1", connection="FailureMode.fm1")
                ├── $RootCause(alias="RootCauseA", connection="OG_1")
                └── $RootCause(alias="RootCauseB", connection="OG_1")

Gate and intermediate-event aliases (e.g. ``OG_1``) are only local identifiers
within the diagram; they do not refer to TRLC records.

A dedicated Rust tool (``puml_cli``'s FTA parser) reads every ``$FailureMode``/
``$RootCause`` in the ``fault_trees``'s ``srcs`` diagrams and generates a
TRLC stub package (``fta_events.trlc``, imported as the ``<fta_package>``
package) with one ``RootCause`` record per reachable ``$RootCause`` alias; each
record's ``failure_modes`` lists the ``FailureMode`` record(s) of the ``$FailureMode``
root it hangs under. No manual linking
step is needed on the TRLC side beyond importing that generated package and
referencing its records from your measure definitions (Step 3).

.. _fta-failure-mode-links:

Linking a ``$FailureMode`` to its failure mode(s)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Pass every ``FailureMode`` fully-qualified name a failure-mode node covers directly
as ``$FailureMode`` arguments. This lets several related failure modes be
consolidated under one failure-mode node and its fault tree, instead of
duplicating the same tree once per failure mode:

.. code-block:: text

    $FailureMode("Message lost, incomplete, or corrupted during transmission",
              "MessagePassing.MessageLost",
              "MessagePassing.MessagePartiallyDelivered",
              "MessagePassing.MessageCorrupted")

For example, the fault tree for ``SampleLibrary.SampleFailureMode`` (the
``FailureMode`` from Step 1) decomposes as follows:

.. uml:: ../_assets/SeoocExample_FTA.puml
   :align: center
   :alt: Example FTA diagram

.. code-block:: text

    @startuml SeoocExample_FTA
    !include fta_metamodel.puml

    $FailureMode("SampleFailureMode takes over the world", "SampleLibrary.SampleFailureMode")

    $OrGate("OG1", "SampleLibrary.SampleFailureMode")

    $IntermediateEvent("SampleFailureMode is Angry", "IEF", "OG1")
    $RootCause("Just bad luck", "JustBadLuck", "OG1")

    $AndGate("AG2", "IEF")
    $RootCause("No More Cookies", "NoMoreCookies", "AG2")
    $RootCause("No More Coffee", "NoMoreCoffee", "AG2")

    @enduml

``JustBadLuck`` alone is sufficient to cause the failure mode (OR gate),
whereas ``NoMoreCookies`` and ``NoMoreCoffee`` must occur together (AND gate)
to cause the intermediate event. Each of these root causes needs a measure in
Step 3.

Step 3 — Choose a countermeasure for every root cause
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Every ``$RootCause`` needs to be addressed by at least one of the following.
Pick the path by *who* closes the root cause:

.. list-table::
   :header-rows: 1
   :widths: 24 46 30

   * - Type
     - Use when the root cause is closed by…
     - Closed via
   * - ``Mitigation``
     - a reasoning why the root cause cannot occur at all (due to system design, ...)
     - ``root_causes`` + mandatory ``justification``
   * - ``CompReq``
     - a runtime/design measure, implemented and tested like any other component requirement.
     - ``derived_from`` referencing the ``RootCause``
   * - ``AoU`` (Assumption of Use)
     - an obligation only the **integrator/caller** can guarantee, not the SEooC.
     - mandatory ``root_causes``

Recording measures (TRLC)
^^^^^^^^^^^^^^^^^^^^^^^^^

Each measure imports the generated ``<fta_package>`` package and references the
``RootCause`` stub(s) it addresses explicitly from its
``root_causes``/``derived_from`` field. For example, the following
``Mitigation`` records (from ``sample_safety_analysis_safetymeasures.trlc``)
address the ``JustBadLuck``, ``NoMoreCookies``, and ``NoMoreCoffee`` root
causes from the FTA diagram in Step 2:

.. code-block:: text

    package SampleLibrary

    import ScoreReq
    import sample_safety_analysis_fta

    ScoreReq.Mitigation JustBadLuck{
        safety = ScoreReq.Asil.B
        description = "Sometimes, the dark side wins. We shall be prepared for that."
        justification = "This root cause is outside our control; no active measure can prevent it."
        version = 1
        root_causes = [sample_safety_analysis_fta.JustBadLuck]
    }

    ScoreReq.Mitigation NoMoreCookies{
        safety = ScoreReq.Asil.B
        description = "We shall only order family size cookie jars"
        justification = "Ordering only family size jars keeps the reserve stable, so this root cause does not apply."
        version = 1
        root_causes = [sample_safety_analysis_fta.NoMoreCookies]
    }

    ScoreReq.Mitigation NoMoreCoffee{
        safety = ScoreReq.Asil.B
        description = "We shall keep a coffee reserve for emergencies"
        justification = "An emergency reserve is kept on hand, so this root cause does not apply."
        version = 1
        root_causes = [sample_safety_analysis_fta.NoMoreCoffee]
    }

The alias ``JustBadLuck`` in the FTA diagram matches the ``RootCause`` stub
``sample_safety_analysis_fta.JustBadLuck`` referenced from the ``Mitigation``'s
``root_causes`` — and likewise for ``NoMoreCookies``/``NoMoreCoffee``. This
explicit reference is how the traceability link is established (there is no
implicit name-matching between the diagram alias and the ``Mitigation``
record's own name).

A ``CompReq``
can reference the generated ``RootCause`` stub directly in its
``derived_from`` list (closing it via a normal, implemented-and-tested
component requirement instead of a dedicated safety measure), or an ``AoU``
can reference it via its mandatory ``root_causes`` field (pushing the
obligation out to the integrator). The traceability report of the enclosing
``dependable_element`` flags a root cause that is covered by none of
``Mitigation``, ``CompReq``, or ``AoU``; with ``maturity = "release"`` this
fails ``bazel test`` of the ``dependable_element``.

An ``AoU`` is how you *push an obligation outward* when the SEooC cannot close a
root cause itself — it must be forwarded to the integrating project (see
:doc:`assumptions_of_use`).

The reverse direction is different: a ``CompReq`` lists an ``AoU`` in
``derived_from`` only for an AoU *received* from another ``dependable_element``
(one of this element's ``deps``). The element's own AoUs are never a
``derived_from`` source; they close a root cause through their own
``root_causes`` field.

Step 4 — Wire the analysis into Bazel
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: starlark
   :caption: bazel/rules/rules_score/examples/seooc/safety_analysis/BUILD

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "failure_modes",
       "fault_trees",
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

   failure_modes(
       name = "sample_failure_modes",
       srcs = ["sample_safety_analysis_failure_modes.trlc"],
   )

   fault_trees(
       name = "sample_fault_trees",
       srcs = [":sample_fta"],
       fta_package = "sample_safety_analysis_fta",
       visibility = ["//visibility:public"],
       deps = [":sample_failure_modes"],
   )

   safety_analysis(
       name = "sample_safety_analysis",
       arch_design = "//design:sample_seooc_design",
       failure_modes = [":sample_failure_modes"],
       fault_trees = ":sample_fault_trees",
       safety_measures = [
           "//docs:sample_aous",
           "sample_safety_analysis_safetymeasures.trlc",
       ],
       visibility = ["//visibility:public"],
   )

The analysis is split in three targets to avoid a dependency cycle: the
``failure_modes`` target holds the ``FailureMode`` records, the ``fault_trees``
target generates the ``<fta_package>`` package with the ``RootCause`` records
referencing them, the measures reference those records, and the
``safety_analysis`` target lists the failure modes and the measures. Every
``AoU`` references a root cause, so each ``assumptions_of_use`` target lists the
``fault_trees`` target in its ``deps`` (so the generated package resolves when
the AoU file is parsed) and is passed in the ``safety_measures`` of the
``safety_analysis`` (so it counts as a measure of the root cause):

.. code-block:: starlark
   :caption: bazel/rules/rules_score/examples/seooc/docs/BUILD

   assumptions_of_use(
       name = "sample_aous",
       srcs = ["aous.trlc"],
       deps = ["//safety_analysis:sample_fault_trees"],
   )

Raw ``.trlc`` files in ``safety_measures`` may only hold ``Mitigation``
records, and a root cause whose failure mode is not in ``failure_modes`` fails
the build.

The ``dependable_element`` derives its own AoUs from the ``safety_measures`` of its
safety analyses, so the AoU reaches the dependees once, however many safety
analyses list it. An ``assumptions_of_use`` target that no ``safety_analysis``
lists in ``safety_measures`` is neither rendered nor forwarded.

The ``dependability_analysis`` target of the dependable element then
references a ``safety_analysis`` target by label:

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

The dependable element stages the pages of each ``dependability_analysis`` under
its name as ``safety_analysis.rst`` and ``fta.puml``. List one
``safety_analysis`` per ``dependability_analysis``; for several safety analyses,
use several ``dependability_analysis`` targets in the ``dependable_element``.
Items shared by several safety analyses (e.g. the same AoU) are kept once.

For the complete attribute reference, see
:ref:`failure_modes <rule-failure-modes>`,
:ref:`fault_trees <rule-fault-trees>`,
:ref:`safety_analysis <rule-safety-analysis>` and
:ref:`dependability_analysis <rule-dependability-analysis>` in the rule index.

Traceability Validation
------------------------

Running ``bazel test`` on the ``dependability_analysis`` target (e.g.
``bazel test //:sample_dependability_analysis`` in the SEooC example) executes
a traceability check that validates the chain from the public API down to the
root causes:

::

          public_api interface ← FailureMode.interface
                                            |
                                        $FailureMode
                                            |
                                     AND / OR gate(s)
                                            |
                                       $RootCause

The check fails if:

- A ``$FailureMode`` references a failure-mode fully-qualified name that does not match any ``FailureMode`` record
- A ``$RootCause`` alias is not a plain TRLC identifier (dotted ``Package.Name`` aliases are rejected for root causes)
- A ``FailureMode`` is defined but not referenced by any ``RootCause`` (no ``$RootCause`` hangs under a ``$FailureMode`` covering it)

The last link of the chain is checked by the traceability report of the
enclosing ``dependable_element``:

::

                                       $RootCause
                                            |
                             Mitigation / AoU / CompReq

A root cause that is not referenced by any ``Mitigation.root_causes``,
``AoU.root_causes`` (from an ``assumptions_of_use`` target in ``safety_measures``), or ``CompReq.derived_from``
is reported as *missing reference to Component Requirements or Safety Measures*.

Fixing a traceability error means ensuring the reference is explicit: the
``Mitigation``/``CompReq``/``AoU`` must import the generated ``<fta_package>``
package (the ``fta_package`` of the ``fault_trees`` target) and reference the
``RootCause`` stub by name (e.g.
``sample_safety_analysis_fta.JustBadLuck``).
