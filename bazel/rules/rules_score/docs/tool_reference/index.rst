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

Tool Reference
==============

Per-tool documentation for the executables that :doc:`../tooling_architecture`
wires together. Each page below is the tool's own ``README.md`` rendered into
this documentation — the README next to the tool source stays the single source
of truth.

.. toctree::
   :maxdepth: 1
   :caption: Workflow

   manual_analysis

.. toctree::
   :maxdepth: 1
   :caption: Requirements

   ai_checker
   trlc

.. toctree::
   :maxdepth: 1
   :caption: Architecture

   plantuml_parser
   validation_core
   Clickable PlantUML <clickable_plantuml>
   bazel

.. toctree::
   :maxdepth: 1
   :caption: Tracing

   lobster_bazel
   lobster
   req_coverage
