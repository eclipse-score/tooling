..
   Copyright (c) 2026 Contributors to the Eclipse Foundation

   See the NOTICE file(s) distributed with this work for additional
   information regarding copyright ownership.

   This program and the accompanying materials are made available under the
   terms of the Apache License Version 2.0 which is available at
   https://www.apache.org/licenses/LICENSE-2.0

   SPDX-License-Identifier: Apache-2.0

Tool Qualification
==================

This section documents the tool qualification of ``rules_score`` following
ISO 26262 principles. The qualification chain is:

.. code-block:: text

   Use Cases → Potential Errors → Tool Requirements → Source Code / Tests

Use cases define the goals users achieve with the tool. Potential errors
describe hypothetical bugs that could impact those use cases. Tool
requirements specify what the tool shall do to mitigate potential errors,
and are traced to source code (via lobster-trace tags) and test cases
(via record properties).

Requirements
------------

.. toctree::
   :maxdepth: 2

   requirements/requirements_rst

Traceability Report
-------------------

.. toctree::
   :maxdepth: 2

   requirements/traceability_rst/index
