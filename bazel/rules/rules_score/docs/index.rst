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

SCORE Rules for Bazel
=====================

``rules_score`` provides Bazel build rules for structuring and documenting
safety-critical software according to S-CORE process guidelines. It covers
the full artefact lifecycle — from requirements and architecture through
safety analysis to the top-level SEooC assembly.

.. toctree::
   :maxdepth: 2
   :caption: Overview

   overview

.. toctree::
   :maxdepth: 2
   :caption: Usage

   user_guide/index
   rule_reference

.. toctree::
   :maxdepth: 2
   :caption: Validation

   tool_reference/specs/bazel_component
   tool_reference/specs/class_design_implementation
   tool_reference/specs/component_internal_api
   tool_reference/specs/component_sequence
   tool_reference/specs/sequence_internal_api

.. toctree::
   :maxdepth: 2
   :caption: Development

   integration_guide
   tooling_architecture
   tool_reference/index

.. toctree::
   :maxdepth: 2
   :caption: Tool Qualification

   Requirements <tool_qualification>
   Traceability Report <requirements/traceability_rst/index>
   quality_report
