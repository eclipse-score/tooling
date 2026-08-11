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
The optional ``mitigates`` field describes (as a free-form string) the hazard
or risk that is mitigated when this assumption is fulfilled.

Traceability to requirements is established at the Bazel level via the ``deps``
attribute on the ``assumptions_of_use`` rule — there is no TRLC ``derived_from``
or ``satisfies`` field on ``AoU`` itself. A dependent component requirement can,
however, declare that it implements a received AoU by referencing it from its own
``derived_from`` field (see `AoU Forwarding`_ below).

.. code-block:: text
   :caption: examples/seooc/docs/aous.trlc

    package SampleType

    import ScoreReq

    ScoreReq.AoU SampleAoU {
        description = "It shall be made sure that this SampleAoU never ends up anywhere"
        safety      = ScoreReq.Asil.B
        mitigates   = "ShmemCreatedWrongName"
        version     = 1
    }

.. code-block:: starlark
   :caption: examples/seooc/docs/BUILD and examples/seooc/BUILD

   assumptions_of_use(
       name = "sample_aous",
       srcs = ["aous.trlc"],
   )

   dependable_element(
       name = "safety_software_seooc_example",
       assumptions_of_use = ["//docs:sample_aous"],
       ...
   )

AoU Forwarding
--------------

When a dependable element depends on another via ``deps``, all **assumptions of
use** defined by the dependency are automatically forwarded to the dependee.
This ensures the integrating project is made aware of every condition it must
satisfy — even those originating from transitive dependencies.

There are two forwarding mechanisms:

**Automatic forwarding (own AoUs)**
All AoUs declared in a dependable element's ``assumptions_of_use`` attribute are
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
      - aou_id: "OtherLibrary.TimingConstraint"
        justification: >
          This SEooC is a library component and has no control over the
          invocation cycle time. The system integrator must ensure that
          calls to the library do not exceed the 10ms cycle time constraint
          imposed by the underlying other_seooc dependency.

**Handling AoUs received in the dependee**
Every AoU a dependable element receives appears as
an item in a "Received AoUs" tier in the dependee's lobster traceability
report. Each received AoU must be covered by exactly one of:

- **Handling it locally**: a component requirement's ``derived_from`` field
  references the AoU it implements (see below). This shows up as "Component
  Requirements" coverage in the report.
- **Chain-forwarding it further** (with justification) via ``aou_forwarding``,
  to be handled by this element's own dependees instead. This shows up as
  "Forwarded AoUs" coverage in the report.

If a received AoU is neither handled nor forwarded, the ``bazel test``
traceability check fails.

A single dependable element can do all three at once — receive AoUs from its
own dependencies, handle some of them locally, chain-forward the rest, and
still contribute its own AoUs to the mix:

.. uml:: ../_assets/aou_forwarding_one_seooc.puml

**Handling a received AoU with a component requirement**
Add a typed, versioned reference to the AoU (``Package.RecordName@version``,
matching the upstream ``AoU`` TRLC record) to the ``derived_from`` field of
the ``CompReq`` that implements it, alongside any ``FeatReq``/
``AssumedSystemReq`` references — all three item kinds share the same field.
Two things are required for the reference to resolve:

1. ``import`` the AoU's package, same as any other TRLC cross-reference.
2. List the ``assumptions_of_use`` target that defines (or, for a received/
   forwarded AoU, originally defined) the record in the
   ``component_requirements`` target's ``deps``. This target provides
   TrlcProviderInfo, so it can be listed directly -- no intermediate wrapper
   is needed.

.. code-block:: text
   :caption: examples/integrator/docs/requirements/component_requirements.trlc

    package IntegratorComponent

    import ScoreReq
    import Integrator
    import SampleType

    ScoreReq.CompReq COMP_INT_001 {
        description = "The startup module shall call the SEooC initialization routine before entering the main loop"
        safety = ScoreReq.Asil.B
        derived_from = [Integrator.FEAT_INT_001@1, SampleType.SampleAoU@1]
        version = 1
    }

.. code-block:: starlark
   :caption: examples/integrator/docs/requirements/BUILD

   component_requirements(
       name = "component_requirements",
       srcs = ["component_requirements.trlc"],
       testonly = True,
       deps = [
           ":feature_requirements",
           "@seooc//docs:sample_aous",
           "@some_other_library//:other_library_aous",
       ],
   )

Being a real TRLC reference, an AoU entry in ``derived_from`` is resolved (and
a typo or an AoU this element does not actually receive is rejected) by the
TRLC parser itself at build time, not by a later lobster-report matching step
-- while the resulting lobster item is still tagged and traced exactly as
before, so the coverage report is unaffected.

**Example: three-level forwarding chain** (the real working code for this
example lives in ``examples/some_other_library``, ``examples/seooc``, and
``examples/integrator``)

::

    other_seooc                     → defines AoU: OtherLibrary.TimingConstraint
        ↑ (deps)
    safety_software_seooc_example   → defines own AoU: SampleType.SampleAoU (auto-forwarded)
                                     → chain-forwards received TimingConstraint via aou_forwarding.yaml
        ↑ (deps)
    integrator_seooc                → receives SampleType.SampleAoU (auto-forwarded)
                                       and OtherLibrary.TimingConstraint (chain-forwarded)
                                     → handles both locally via derived_from (no further dependees)

.. code-block:: starlark
   :caption: examples/seooc/BUILD

   dependable_element(
       name = "safety_software_seooc_example",
       assumptions_of_use = ["//docs:sample_aous"],
       aou_forwarding = "aou_forwarding.yaml",
       deps = ["@some_other_library//:other_seooc"],
       ...
   )
