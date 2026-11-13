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

Assumptions of Use
===================

Conditions that the *integrating project* must satisfy when using your SEooC.
An own ``AoU`` pushes the obligation to prevent FTA root causes out to the
integrating project. A ``RootCauseAssessment`` links it to the assessed root
causes through ``measures``; the ``AoU`` itself does not reference a root cause.

Traceability to requirements is established at the Bazel level via the ``deps``
attribute on the ``assumptions_of_use`` rule — there is no TRLC ``derived_from``
or ``satisfies`` field on ``AoU`` itself. A received AoU is already a requirement:
manual analysis references it directly to provide local verification evidence
(see `AoU Forwarding`_ below).

.. code-block:: text
   :caption: examples/seooc/docs/aous.trlc

    package SampleType

    import ScoreReq

    ScoreReq.AoU SampleAoU {
        description = "It shall be made sure that shared memory segments are never created with the wrong name (ShmemCreatedWrongName)"
        safety      = ScoreReq.Asil.B
        version     = 1
    }

.. code-block:: starlark
   :caption: examples/seooc/docs/BUILD

   assumptions_of_use(
       name = "sample_aous",
       srcs = ["aous.trlc"],
   )

The ``dependable_element`` has no AoU attribute: its own AoUs are the
``assumptions_of_use`` targets that the ``safety_analysis`` targets of its
``dependability_analysis`` list in ``safety_measures`` (see below).

Preventing a root cause
-----------------------

To record an own ``AoU`` as a preventive measure, reference it, with its
version, in the ``measures`` of a ``RootCauseAssessment``. The assessment's
``rationale`` should explain why fulfilment of the referenced AoUs is
sufficient to address the assessed root causes.

Listing an AoU records the measure, not evidence that its obligation has been
fulfilled. Its effectiveness depends on the integrating project actually
satisfying the stated condition. The reference is recorded as follows:

.. code-block:: text
   :caption: assessments.trlc

    package SampleLibrary

    import ScoreReq
    import SampleType
    import sample_safety_analysis_fta

    ScoreReq.RootCauseAssessment UserConfigurationAssessed {
        root_causes = [sample_safety_analysis_fta.UserProvidedWrongConfiguration]
        measures    = [SampleType.SampleAoU@1]
        safety      = ScoreReq.Asil.B
        rationale   = "The user provides a correct configuration."
        version     = 1
    }

The ``safety_analysis`` target lists the ``assumptions_of_use`` target in
``safety_measures`` and the assessments in ``assessments``. The
``assumptions_of_use`` target needs no ``fault_trees`` dep:

.. code-block:: starlark

   assumptions_of_use(
       name = "sample_aous",
       srcs = ["aou.trlc"],
   )

   safety_analysis(
       name = "sample_safety_analysis",
       assessments = ["assessments.trlc"],
       fault_trees = ":sample_fault_trees",
       safety_measures = [":sample_aous"],
       # ...
   )

The ``dependable_element`` derives its own AoUs from the ``safety_measures`` of its
safety analyses: an AoU is exposed to dependees once, however many safety
analyses list it. An ``assumptions_of_use`` target that no ``safety_analysis``
lists in ``safety_measures`` is neither rendered nor forwarded, and a
``dependable_element`` without a safety analysis has no own AoUs.

Neither own nor received AoUs are ``derived_from`` sources of a ``CompReq``.
Component requirements derive only from feature or assumed system requirements.

AoU Forwarding
--------------

When a dependable element depends on another via ``deps``, all **assumptions of
use** defined by the dependency are automatically forwarded to the dependee.
This ensures the integrating project is made aware of every condition it must
satisfy — even those originating from transitive dependencies.

There are two forwarding mechanisms:

**Automatic forwarding (own AoUs)**
All AoUs that the element's safety analyses list in ``safety_measures`` are
automatically forwarded to every element that lists it in ``deps``. No
configuration is needed.

**Chain-forwarding (received AoUs)**
When a dependable element receives forwarded AoUs from its own dependencies, it
can selectively forward them further by providing an ``aou_forwarding`` YAML
file. Each entry requires a mandatory justification explaining *why* this AoU
is forwarded rather than handled locally:

.. code-block:: yaml
   :caption: examples/seooc/aou_forwarding.yaml

    forwarded_aous:
      - aou_id: "OtherLibrary.TimingConstraint@1"
        justification: >
          This SEooC is a library component and has no control over the
          invocation cycle time. The system integrator must ensure that
          calls to the library do not exceed the 10ms cycle time constraint
          imposed by the underlying other_seooc dependency.

``aou_id`` carries the version of the received AoU (``Package.Name@version``),
so a justification is reviewed again when the AoU changes. An unknown ID, a
mismatching version or a duplicate entry fails the build.

**Handling AoUs received in the dependee**
Every AoU a dependable element receives appears as
an item in a "Received AoUs" tier in the dependee's lobster traceability
report. An AoU that reaches the element along several paths (for example
directly and through a dependency that forwards it) appears once; if the
paths carry different definitions, the build fails. Each received AoU must be
covered by at least one of:

- **Handling it locally**: manual analysis references the received AoU directly
    (see below). This shows up as "Manual Analysis" coverage in the report.
- **Chain-forwarding it further** (with justification) via ``aou_forwarding``,
  to be handled by this element's own dependees instead. This shows up as
  "Forwarded AoUs" coverage in the report.

If a received AoU is neither handled nor forwarded, the ``bazel test``
traceability check fails.

A single dependable element can do all three at once — receive AoUs from its
own dependencies, handle some of them locally, chain-forward the rest, and
still contribute its own AoUs to the mix:

.. uml:: ../_assets/aou_forwarding_one_seooc.puml

**Handling a received AoU with manual analysis**
Reference the received AoU directly in the review's ``requirements`` list and
add the review target to ``dependable_element.manual_analysis``. The dependency's
AoU records are already present in the "Received AoUs" report level; there is no
need to copy them into component requirements.

.. code-block:: yaml
   :caption: examples/seooc/manual_analysis/received_input_pointer/analysis.yaml

    requirements:
      - "OtherLibrary.ValidInputPointer@1"

.. code-block:: starlark
   :caption: examples/seooc/BUILD

   dependable_element(
       name = "safety_software_seooc_example",
       manual_analysis = [
           "//manual_analysis/received_input_pointer:review",
           "//manual_analysis/received_space_constraint:review",
       ],
       deps = ["@some_other_library//:other_seooc"],
       # Other required attributes omitted.
   )

Include the original AoU source, implementation and relevant configuration in
the review's contexts so changes invalidate its lock. Run the review test
explicitly to check the lock and recorded assertions. LOBSTER coverage establishes
the reference, not a successful or current verification result. Direct analysis
and chain-forwarding are alternative coverage paths, not simultaneous obligations.
This does not verify that the element's own AoUs have been fulfilled by its users.

**Example: three-level forwarding chain** (the real working code for this
example lives in ``examples/some_other_library``, ``examples/seooc``, and
``examples/integrator``)

::

    other_seooc                     → defines AoUs: OtherLibrary.TimingConstraint,
                                                    OtherLibrary.SpaceConstraint,
                                                    OtherLibrary.ValidInputPointer
        ↑ (deps)
    safety_software_seooc_example   → defines own AoU: SampleType.SampleAoU (auto-forwarded)
                                    → reviews received ValidInputPointer directly with manual analysis
                                    → reviews received SpaceConstraint directly (validation remains unresolved)
                                    → chain-forwards received TimingConstraint via aou_forwarding.yaml
        ↑ (deps)
    integrator_seooc                → receives SampleType.SampleAoU (auto-forwarded)
                                       and OtherLibrary.TimingConstraint (chain-forwarded)
                                    → still requires direct review evidence for both received AoUs

.. code-block:: starlark
   :caption: examples/seooc/BUILD

   dependable_element(
       name = "safety_software_seooc_example",
       aou_forwarding = "aou_forwarding.yaml",
       deps = ["@some_other_library//:other_seooc"],
       ...
   )
