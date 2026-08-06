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


Step 5 — Build
================

During Development you can enable a build with warnings instead of errors for all checks:

``maturity = "development"``

Each example under ``examples/`` (e.g. ``examples/minimal/``) is its own standalone
Bazel module with its own ``MODULE.bazel``, separate from the main ``rules_score``
module. Bazel commands therefore need to be run from within that module's
directory, not from the repository root — the code blocks below include the
``cd`` step so they can be copy-pasted as-is from the repository root.

Run the build from within the ``examples/minimal/`` standalone module:

.. code-block:: bash

   cd bazel/rules/rules_score/examples/minimal
   bazel build //:my_element

Expected output files:

.. code-block:: text

   bazel-bin/my_element_doc/html/          ← Sphinx HTML documentation
