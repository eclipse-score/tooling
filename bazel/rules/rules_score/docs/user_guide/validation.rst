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

Validation
==========

Unit Tests
----------

Unit tests in ``rules_score`` components are written with **GoogleTest** and built
with ``cc_test``. Each test case that covers a component requirement must carry
a ``lobster-tracing`` annotation so that the build can link the test back to the
requirement.

Annotating Tests
~~~~~~~~~~~~~~~~

Call ``RecordProperty`` inside the test body next to the respective code blocks:

.. code-block:: cpp

   TEST(Foo, GetNumber) {
       ::testing::Test::RecordProperty("lobster-tracing",
                                       "SampleComponent.REQ_COMP_001");

       ::testing::Test::RecordProperty("given",
                                       "a default-constructed Foo instance");
       unit_1::Foo unit{};

       ::testing::Test::RecordProperty("when", "GetNumber is called");
       ::testing::Test::RecordProperty("then", "it returns 42");
       EXPECT_EQ(unit.GetNumber(), 42u);
   }

.. list-table::
   :header-rows: 1
   :widths: 20 15 65

   * - Property
     - Required
     - Description
   * - ``lobster-tracing``
     - yes
     - Comma-separated requirement IDs; links the test to one or more ``CompReq`` records
   * - ``given``
     - with coverage lock
     - Initial state / precondition
   * - ``when``
     - no
     - Action or event under test
   * - ``then``
     - with coverage lock
     - Expected outcome

A test case that is part of a coverage lock (see `Stating Coverage for test cases`_)
needs a ``given`` and a ``then``. The ``when`` is always optional.

Deriving Given/When/Then from the Test Name
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

A test named ``[<Prefix>_]Given<X>[_When<Y>]_(Expect|Then)<Z>`` does not need the
``given``/``when``/``then`` properties. Its name provides them. ``<Prefix>`` is
alphanumeric, and ``<X>``, ``<Y>`` and ``<Z>`` are in CamelCase. Each of them becomes
lowercase words separated by spaces:

.. code-block:: cpp

   TEST(Stack, GivenEmptyStack_WhenPopCalled_ExpectError) {
       ::testing::Test::RecordProperty("lobster-tracing",
                                       "SampleComponent.REQ_COMP_001");
       ...
   }

This test has the Given ``empty stack``, the When ``pop called`` and the Then
``error``.

* The ``_When<Y>`` part is optional. ``GivenEmptyStack_ExpectError`` has no When.
* For a value-parameterized test, the segments of the name are separated by ``/``
  and the last segment that matches is used.
* If a test sets any of ``given``, ``when`` or ``then`` to a non-blank value, its
  name is ignored completely, so parts missing from the properties are not filled in
  from the name. If the name also matches the pattern, lobster prints a warning. Set
  all parts that the test needs, or none.
* Deriving requires a ``lobster`` version that provides the option
  ``--derive-gwt-from-test-name``. It is not available in lobster 1.0.6 and earlier,
  where only the properties are used.
* After a ``lobster`` upgrade that provides the option, the derived text of tests that
  follow the pattern and set none of the properties changes. Their entries in the lock
  become stale, so run ``bazel run //<pkg>:<name>.update`` and review the diff.

A ``dependable_element`` report renders every annotated test on its **Unit
Test** page at ``<dependable-element>_index/traceability_report/unit_test.html``.
The entry shows its Given/When/Then text, links to the traced requirements, and
links back to the test source.

A test without ``lobster-tracing`` has no traceability and is not included in
coverage tracking.

Stating Coverage for test cases
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Coverage is declared through a committed ``test_case_coverage.lock.yaml`` file that lists,
per requirement, every test case (uid + given/when/then) that covers it.
Committing the file is stating the coverage claim.

Every test case in the lock needs a Given and a Then, set through properties or
derived from the test name. The When is optional and is stored as an empty string
when absent. A test case without a Given or a Then is reported as an error. With
``maturity = "release"`` on the ``dependable_element`` the error fails the build, with
``"development"`` it is downgraded to a warning (see
:doc:`../tool_reference/test_case_coverage`).

In Bazel the yaml file can be linked to the ``component`` macro via the ``test_case_coverage_lock`` attribute:

.. code-block:: starlark

   component(
       name = "my_component",
       requirements = [":my_component_requirements"],
       components  = [":unit_a", ":unit_b"],
       test_case_coverage_lock = "test_case_coverage.lock.yaml",
   )

Two complementary workflows keep the lock file in sync:

* **``bazel run …update``** — reads the current test results and **rewrites**
  ``test_case_coverage.lock.yaml`` in the source tree.
* **``bazel test //…``** — the build action recomputes coverage from the same
  test results and **compares** it against the committed lock. Any drift (new
  test, removed test, changed GWT text, version bump) fails the build until the
  lock is refreshed and re-committed.

When a ``dependable_element`` report is built, this claim is rendered on its
**Test Case Coverage** page at
``<dependable-element>_index/traceability_report/test_case_coverage.html``.
Each requirement entry shows its coverage status and GWT text, with links to the
individual traced test cases.

For example, the SEooC example's lock records two tests covering one component
requirement:

.. code-block:: yaml

   schema_version: 3
   requirements:
     - id: SampleComponent.REQ_COMP_001
       version: '1'
       description: The numeric value management interface shall provide a read operation that returns a uint8_t value
       test_cases:
         - uid: //Foo:GetNumber
           given: a default-constructed Foo instance
           when: GetNumber is called
           then: it returns 42
         - uid: //Foo:GetNumberViaConstInstance
           given: a const default-constructed Foo instance
           when: GetNumber is called through a const reference
           then: it still returns 42

For the full tool description — lock file format, update workflow, design
decisions — see :doc:`../tool_reference/test_case_coverage`.
