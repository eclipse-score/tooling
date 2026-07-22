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

Run the build from within the ``examples/minimal/`` standalone module:

.. code-block:: bash

   bazel build //:my_element

Expected output files:

.. code-block:: text

   bazel-bin/my_element_doc/html/          ← Sphinx HTML documentation

When integrating the element into your own workspace, reference it by its full
package label:

.. code-block:: bash

   bazel build //my/package:my_element

Run all validations (architecture consistency, scope checks, traceability):

.. code-block:: bash

   bazel test //my/package:my_element
