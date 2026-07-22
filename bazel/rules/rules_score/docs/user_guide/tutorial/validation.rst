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

Step 4 — Validation
=====================

Test targets can be defined on different architectural levels. They are attached to ``unit``, ``component``, and
``dependable_element`` via the ``tests`` attribute. On  ``component`` and ``dependable_element`` level, the test
should focus more on integration and system testing, while on ``unit`` level, the test should focus on unit testing.


``rules_score`` does not require a separate test specification document.
Instead, test intent is captured as a **Given-When-Then** description right
next to the code as record properties in the test name/body itself. The specification will
then be rendered in the traceability report, together with the test results and coverage information.

test/my_unit_test.cpp
----------------------

.. literalinclude:: ../../examples/minimal/test/my_unit_test.cpp
   :language: cpp
   :lines: 14-

Each ``RecordProperty("lobster-tracing", "...")`` call names the requirement
identifiers covered by that test case.

BUILD
------

.. code-block:: starlark

   cc_test(
       name = "my_unit_test",
       srcs = ["test/my_unit_test.cpp"],
       deps = [
           ":my_unit_lib",
           "@googletest//:gtest_main",
       ],
   )

   unit(
       name = "MyUnit",
       implementation = [":my_unit_lib"],
       scope = ["//:my_unit_lib"],
       unit_design = [":MyUnit_design"],
       tests = [":my_unit_test"],
   )

Run the tests with:

.. code-block:: bash

   bazel test //:my_unit_test

→ Full guide: :doc:`../unit_design`
