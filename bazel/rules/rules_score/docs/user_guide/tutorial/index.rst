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


Tutorial: Your First Dependable Element
========================================

This tutorial walks you through building a minimal
**Safety Element out of Context (SEooC)** step by step.  All examples are taken
from the standalone module at
`bazel/rules/rules_score/examples/minimal/ <https://github.com/eclipse-score/tooling/tree/main/bazel/rules/rules_score/examples/minimal>`_
(standalone module) — you can run each step there with
``bazel build //:my_element``.

By the end you will have a fully validated SEooC with requirements, a static
architecture diagram, a unit design, and a passing build.

Workflow
--------

The tutorial follows the S-CORE development flow top-down: specify *what* the
element must do, design *how* it is structured, implement and verify it, and let
the build check that every artefact stays consistent.

::

    Step 1              Step 2                 Step 3            Step 4          Step 5
    Requirements   →    SW Architectural   →   Unit Design  →   Validation  →   Build
                        Design
    ───────────         ───────────────        ───────────      ──────────      ─────────
    AssumedSystemReq    static diagram         class/sequence   unit &          bazel build
    FeatReq             (components + units)    diagram per      component       (runs every
    CompReq             →  Bazel targets        unit  →  code    tests +         consistency
    (.trlc records)     modelled after it       it validates    lobster-        check)
                                                against          tracing

Each step builds on the previous one, and each has an automatic check:

1. **Requirements** — write ``AssumedSystemReq`` / ``FeatReq`` / ``CompReq`` TRLC
   records and wire their Bazel targets. Traceability is type-checked by
   ``trlc --verify``. → :doc:`requirements`
2. **SW Architectural Design** — draw the static PlantUML diagram naming every
   component and unit, then model it 1:1 as ``dependable_element`` / ``component``
   / ``unit`` targets. The diagram is the design; the Bazel model follows it, and
   the architecture-consistency check enforces the match. → :doc:`architecture`
3. **Unit Design** — add a class/sequence diagram for each unit; it is validated
   against the real C++ implementation. → :doc:`unit_design`
4. **Validation** — attach unit/component/system tests and annotate them with
   ``lobster-tracing`` to link test cases back to requirements. → :doc:`validation`
5. **Build** — run ``bazel build //:my_element`` to execute all checks at once and
   assemble the documentation. → :doc:`build`

.. toctree::
   :maxdepth: 1

   requirements
   architecture
   unit_design
   validation
   build
