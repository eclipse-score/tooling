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
   * - ``test-spec``
     - no
     - Bazel label of a ``test_specification`` target; replaces ``given``/``when``/``then``
       (see `Formal Test Specification (RST)`_)

A test case that is part of a coverage lock (see `Stating Coverage for test cases`_)
needs a ``given`` and a ``then``, or a ``test-spec``. The ``when`` is always optional.

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
The entry shows its Given/When/Then text (or its ``test-spec`` reference), links to
the traced requirements, and links back to the test source.

A test without ``lobster-tracing`` has no traceability and is not included in
coverage tracking.

Formal Test Specification (RST)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Given/When/Then fits short unit tests. For more complex scenarios, such as
multi-step or higher-order tests, a test case can reference a formal test
specification written in RST instead. The specification lives in its own file, is
reviewed like any other document, and its content is part of the coverage lock.

One RST file holds exactly one test specification. Several test cases may reference
the same specification. The file has this structure (the underline characters are
free to choose, comments such as a copyright header are allowed):

.. code-block:: rst

   Read operation returns a stable value
   =====================================

   Test Goal
   ---------
   Show that the read operation returns the expected value, independent of how
   often and through which kind of reference it is called.

   Test Focus
   ----------
   - interface behavior of ``Foo::GetNumber``
   - repeated reads through mutable and const references

   Test Steps
   ----------

   Step 1: First read
   ~~~~~~~~~~~~~~~~~~
   :Precondition: A default-constructed ``Foo`` instance exists.
   :Action: Call ``GetNumber`` once.
   :Expected: The returned value is 42.

   Step 2: Repeated read through a const reference
   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
   :Action: Call ``GetNumber`` again through a const reference.
   :Expected: The returned value is still 42.

.. list-table::
   :header-rows: 1
   :widths: 25 15 60

   * - Section / field
     - Required
     - Description
   * - Title
     - yes
     - Exactly one top-level section; its subsections are the ones below
   * - ``Test Goal``
     - yes
     - What the test shall show
   * - ``Test Focus``
     - no
     - Aspects the test concentrates on, e.g. fault injection or boundary values
   * - ``Test Steps``
     - yes
     - One subsection per step (at least one), each with a field list
   * - ``:Precondition:``
     - no
     - State that must hold before the step
   * - ``:Action:``
     - yes
     - What the step does
   * - ``:Expected:``
     - yes
     - The expected outcome of the step

Any other content (unknown sections or fields, text outside the sections) is an
error, so that everything a reviewer reads in the file also ends up in the lock.

Declare the file with a ``test_specification`` target and list it in the
``test_specs`` of the unit whose tests reference it:

.. code-block:: starlark

   test_specification(
       name = "get_number_spec",
       src = "get_number_spec.rst",
   )

   unit(
       name = "unit_1",
       ...
       tests = [":unit_1_test"],
       test_specs = [":get_number_spec"],
   )

A test case references the specification by its Bazel label with the
``test-spec`` property, spelled as in the unit's ``test_specs``. A package-relative
entry such as ``":get_number_spec"`` in ``//unit_1`` is spelled
``//unit_1:get_number_spec``:

.. code-block:: cpp

   TEST(Foo, GetNumberRepeatedly) {
       ::testing::Test::RecordProperty("lobster-tracing",
                                       "SampleComponent.REQ_COMP_001");
       ::testing::Test::RecordProperty("test-spec", "//unit_1:get_number_spec");
       ...
   }

* The RST format is checked when the ``test_specification`` target, or a unit that
  lists it, is built. A malformed file fails the build, independent of ``maturity``.
* A ``test-spec`` that does not resolve to a ``test_specification`` in the
  ``test_specs`` of the component's units is reported like a missing Given/Then.
* A specification replaces Given/When/Then completely. If the test also sets
  ``given``/``when``/``then`` or its name follows the Given/When/Then pattern, that
  text is ignored and a warning is printed.
* The ``test_specification`` may live in another package (grant ``visibility`` to
  the unit's package) or in another repository. A spec of another repository is
  listed and referenced with the repository name the unit's module uses, for example
  ``@spec_repo//specs:conn_fault``.
* References do not depend on canonical repository names. The spelling stays valid
  when the SEooC is built as a dependency of another module.
* If a reference does not match a ``test_specs`` entry, the label of the
  ``test_specification`` within its own repository (``//pkg:name``) or of its RST
  file (``//pkg:name.rst``) is tried, unless two specifications of the component
  share it.

The traceability report shows the specification with its goal, focus and a table
of the steps.

Stating Coverage for test cases
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Coverage is declared through a committed ``test_case_coverage.lock.yaml`` file that lists,
per requirement, every test case (uid + given/when/then, or its formal test
specification) that covers it. Committing the file is stating the coverage claim.

Every test case in the lock needs a Given and a Then, set through properties or
derived from the test name, or a resolvable ``test-spec``. The When is optional and
is stored as an empty string when absent. A test case without a Given or a Then and
without a ``test-spec`` is reported as an error. With
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
  test, removed test, changed GWT text or test specification, version bump) fails
  the build until the lock is refreshed and re-committed.

When a ``dependable_element`` report is built, this claim is rendered on its
**Test Case Coverage** page at
``<dependable-element>_index/traceability_report/test_case_coverage.html``.
Each requirement entry shows its coverage status and the GWT text or test
specification, with links to the individual traced test cases.

For example, the SEooC example's lock records three tests covering one component
requirement. ``GetNumberRepeatedly`` references a formal test specification, so
its entry holds a ``spec`` instead of ``given``/``when``/``then``:

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
         - uid: //Foo:GetNumberRepeatedly
           spec:
             ref: //unit_1:get_number_spec
             goal: Show that the read operation of the numeric value management interface returns the expected uint8_t value, independent of how often and through which kind of reference it is called.
             focus: |-
               - interface behavior of Foo::GetNumber
               - repeated reads through mutable and const references
             steps:
               - precondition: A default-constructed Foo instance exists.
                 action: Call GetNumber once.
                 expected: The returned value is 42.
               - precondition: ''
                 action: Call GetNumber again through a const reference to the same instance.
                 expected: The returned value is still 42.
         - uid: //Foo:GetNumberViaConstInstance
           given: a const default-constructed Foo instance
           when: GetNumber is called through a const reference
           then: it still returns 42

For the full tool description — lock file format, update workflow, design
decisions — see :doc:`../tool_reference/test_case_coverage`.
