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

Tool Qualification
==================

This section documents the tool qualification of ``rules_score`` following
ISO 26262 principles. The qualification chain is:

.. code-block:: text

   Use Cases → Potential Errors → Tool Requirements → Source Code / Tool Tests

Use cases define the goals users achieve with the tool. Potential errors
describe hypothetical bugs that could impact those use cases. Tool
requirements specify what the tool shall do to mitigate potential errors,
and are traced to source code (via lobster-trace tags) and tool tests.

Tool tests are the Bazel tests of ``rules_score`` that carry a
``lobster-tracing=rules_score.<Requirement>`` tag. The ``tool_traceability``
macro in ``bazel/rules/rules_score/test`` collects them, reports each as a
test case in ``tool_tests.lobster`` and runs the traceability check
(``tool_traceability_test``) over the whole chain.

Requirements
------------

.. toctree::
   :maxdepth: 1

   Use Cases <requirements/use_cases_rst>
   Potential Errors <requirements/potential_errors_rst>
   Tool Requirements <requirements/tool_requirements_rst>
   Validation Core Requirements <requirements/validation_core_requirements_rst>
