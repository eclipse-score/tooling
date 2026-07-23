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
or ``satisfies`` field on ``AoU``.

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

**Handling forwarded AoUs in the dependee**
Forwarded AoUs appear as a "Forwarded AoUs" tier in the dependee's lobster
traceability report. The dependee must handle each forwarded AoU by one of:

- Linking it to a component requirement that addresses the assumption
- Linking it to a test that verifies the assumption is met
- Chain-forwarding it further (with justification) to its own dependees

If a forwarded AoU is not handled, the ``bazel test`` traceability check will fail.

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
                                       and OtherLibrary.TimingConstraint (chain-forwarded), must handle both

.. code-block:: starlark
   :caption: examples/seooc/BUILD

   dependable_element(
       name = "safety_software_seooc_example",
       assumptions_of_use = ["//docs:sample_aous"],
       aou_forwarding = "aou_forwarding.yaml",
       deps = ["@some_other_library//:other_seooc"],
       ...
   )
