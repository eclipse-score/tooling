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

.. toctree::
   :maxdepth: 1

   requirements
   architecture
   unit_design
   validation
   build
