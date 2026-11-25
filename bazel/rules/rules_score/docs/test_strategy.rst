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

Test Strategy
=============

Developer-facing description of how ``rules_score`` is tested. Tests are
layered; every behaviour is asserted at the lowest level that can observe it.

Levels
------

.. list-table::
   :header-rows: 1
   :widths: 12 30 14 22 22

   * - Level
     - Location
     - Module
     - Framework
     - Run
   * - L0 tool unit
     - ``bazel/rules/rules_score/src/tests/``, ``bazel/rules/rules_score/src/test_case_coverage/``, ``plantuml/parser/``
     - root
     - ``py_test`` (pytest), ``rust_test``
     - ``bazel test //bazel/rules/rules_score/src/... //plantuml/parser/...``
   * - L1 Starlark logic
     - ``bazel/rules/rules_score/private/tests/``
     - root
     - skylib ``unittest`` / ``loadingtest``
     - ``bazel test //bazel/rules/rules_score/private/tests/...``
   * - L2 analysis contract
     - ``bazel/rules/rules_score/test/analysis/<rule>/``
     - ``test/``
     - skylib ``analysistest`` (existing), ``rules_testing`` (new)
     - ``cd bazel/rules/rules_score/test && bazel test //analysis/...``
   * - L3 artifacts
     - ``bazel/rules/rules_score/test/artifacts/<rule>/``
     - ``test/``
     - golden files (``write_source_files``), assertions on action outputs
     - ``cd bazel/rules/rules_score/test && bazel test //artifacts/...``
   * - L4 end-to-end
     - ``bazel/rules/rules_score/integration_test/``
     - own module
     - ``rules_bazel_integration_test`` (nested Bazel)
     - ``cd bazel/rules/rules_score/integration_test && bazel test //...``

``test/`` and ``integration_test/`` are separate Bazel modules (ignored by the
root workspace, consuming ``score_tooling`` via ``local_path_override``), so they
are run from their own directory and have their own CI jobs. L1 runs in the root
CI step ``//bazel/rules/rules_score/... -//bazel/rules/rules_score/examples/...``;
the L0 tools run there (``src/``) and in the ``//plantuml/...`` step (parsers).
The legacy ``test/BUILD`` stays in place; its tests move into the level
directories rule by rule. ``analysis/dependable_element`` is the template for
new L2 tests, ``analysis/rst_to_trlc`` and ``artifacts/rst_to_trlc`` show the
single-action case, ``artifacts/component`` is the template for new L3 tests,
the L1 tests below are the template for new L1 tests.

L2 conventions
--------------

* One ``<rule>_test.bzl`` per rule in ``test/analysis/<rule>/``, exposing a
  ``<rule>_test_suite(name)`` macro that is instantiated in that directory's
  ``BUILD`` (``rules_testing`` ``analysis_test`` + ``test_suite``). Each test is
  a ``_<case>_test(name)`` function creating its fixtures and an
  ``_<case>_impl(env, target)`` function with the assertions.
* Shared helpers live in ``test/lib/``: ``analysis.bzl`` (``find_action``,
  ``argv_of``, ``flag_value``, ``file_write_json``, ``output_path``,
  ``expect_maturity_flags``, ``expect_failure_message``) and ``fixtures.bzl``
  (minimal ``manual`` fixtures such as ``fixture_dependable_element``, built
  with ``util.helper_target``). Derive fixture labels from the subject's label,
  not from literal names.
* The subject is the target that owns the actions, for example
  ``<name>_index`` of a ``dependable_element``. Pick actions by mnemonic and
  output name, not by position, and assert flags by presence or absence
  (``flag_value`` for flag/value pairs).
* Positive cases check providers, action arguments and ``FileWrite`` content.
  Negative cases use ``expect_failure = True`` for ``fail()`` paths reachable at
  analysis time; a ``development`` fixture asserts that the same input does not
  fail (the warning itself is not observable). Execution-time failures
  (validation CLI, lobster, coverage lock check) are not observable here; they
  belong to L0 and L4.
* Fixtures may violate release gates if the tested contract is analysis-only.
  This is allowed because they are ``manual`` helper targets that are never
  built.
* ``--analysis_testing_deps_limit`` only applies to tests that use a
  configuration transition (``config_settings``) and counts the direct edges of
  the test rule (4 for a ``dependable_element`` subject), so the default limit
  is no obstacle.
* Migrating a legacy skylib ``analysistest`` from ``test/``: port the assertions
  into the matching ``analysis/<rule>/`` suite, check the old targets with
  ``bazel query 'rdeps(//..., <old target>)'``, then delete the old rule, its
  ``BUILD`` targets, suite entries and fixtures in the same change.

L3 conventions
--------------

* One package per rule in ``test/artifacts/<rule>/``. Its ``BUILD`` builds a
  real chain (requirements → rule) and diffs the output against a checked-in
  golden. ``component`` and ``dependable_element`` are test rules: tag them
  ``manual`` so ``bazel test //...`` only runs the golden and log checks.
* Helpers live in ``test/lib/artifacts.bzl``: ``golden_test`` (wraps
  ``write_source_files``; its ``<name>`` target refreshes the golden, it is not
  a test itself), ``output_file`` (exposes the one file whose path ends in
  ``suffix`` from ``DefaultInfo`` or an output group) and
  ``component_architecture_lobster`` (reads ``ComponentInfo.architecture``).
  They are ``testonly``; no output group is added to the shipped rules for
  tests.
* Name goldens ``<output>.golden``. The suffix keeps formatters and the header
  check off generated content.
* ``bazel run //artifacts/<rule>:<name>`` refreshes a golden,
  ``//artifacts/<rule>:<name>_test`` diffs it. Review the golden diff like code.
* Execution-time logs (for example validation PASS/FAIL) are read with
  ``output_file`` from the ``debug`` output group and checked by a small
  ``sh_test``; they are not goldens.

L1 conventions
--------------

* One ``<module>_test.bzl`` per tested ``private/<module>.bzl`` in
  ``private/tests/``, exposing a ``<module>_test_suite(name)`` macro that is
  instantiated in ``private/tests/BUILD``. The ``l1`` test suite aggregates all.
* Tests use skylib ``loadingtest``: one test target per ``loadingtest.equals``
  case, named ``<suite>_<case>``. Only functions that need no real ``ctx`` or
  action belong here; fake files and contexts with ``struct(...)``.
* ``@score_tooling`` is the main repository in the root module. For
  external-repository cases use a dependency label such as ``@bazel_skylib//lib:...``
  and assert that its ``workspace_name`` is not empty.
* ``fail()`` paths and anything observing providers or actions are tested at L2.

L0 Python conventions
---------------------

* One ``<tool>_test.py`` per tool in ``src/tests/``, declared with the
  ``pytest_test`` macro (``src/tests/pytest_test.bzl``), which runs pytest on
  the file, writes the JUnit XML and maps ``--test_filter`` to ``-k``. Tests are
  plain ``unittest.TestCase`` classes run by pytest; there is no
  ``unittest.main()``. The ``tests`` test suite aggregates all.
* Data files are passed as ``$(rlocationpath ...)`` env vars and resolved with
  ``python.runfiles``, not relative paths.
* ``src/tests/`` is a separate package so that the non-recursive ``src/*.py``
  globs (``:py_srcs``, traceability) stay unchanged.
* Lint gate: ruff and ty must be clean for ``src/tests/...``::

    bazel build --config=lint --@aspect_rules_lint//lint:fail_on_violation \
        //bazel/rules/rules_score/src/tests/...

  The "Python lint" workflow runs this as a blocking step; the tools in ``src/``
  are not yet covered by the gate.

Level responsibilities
----------------------

.. list-table::
   :header-rows: 1
   :widths: 14 26 30 30

   * - Level
     - Observes
     - Asserts
     - Does not assert
   * - L0
     - Tool process with hand-written inputs
     - Parser/checker logic, exit codes, messages on broken input
     - Rule wiring
   * - L1
     - Pure ``.bzl`` functions
     - Label keys, scope calculation, config building, view layout, provenance
     - Providers, actions
   * - L2
     - Providers, actions, ``FileWrite`` content, analysis errors
     - Action arguments (maturity flags), ``validation_inputs.json`` and
       ``architecture.json`` content, ``expect_failure`` messages
     - Anything an action produces at execution time
   * - L3
     - Real action outputs of one in-process build
     - Goldens for contract outputs, targeted assertions for the rest
     - User workflows, bzlmod consumption
   * - L4
     - A consumer's ``bazel build`` / ``bazel test``
     - Outcome (pass/fail) and stderr pattern per case, ``.update`` workflows,
       lockfile, toolchain/extension consumption
     - Output content (covered by L3)

Example, "release gate fails the build": L0 checks that the tool exits non-zero
on broken input, L2 checks that no ``--warn-on-errors`` is passed under
``maturity = "release"``, L4 checks that one broken fixture fails with the
expected message. Each level covers a different link of the chain.

Golden policy
-------------

Diff tests are used only at L3 and only for structured, execution-time contract
outputs (for example ``*_architecture.lobster``). They are not used for
pass/fail logs, HTML, or content already readable at L2
(``validation_inputs.json``, ``architecture.json``). Goldens are declared with
``write_source_files`` (``aspect_bazel_lib``): ``bazel run`` of the ``<name>``
target refreshes the golden, ``<name>_test`` diffs it. The ``.golden`` suffix
keeps formatters and header checks off generated content.

Fixture policy
--------------

* Positive fixtures use ``maturity = "release"``: any finding fails their
  verdict, so previously hidden errors surface.
* Negative fixtures use ``maturity = "development"`` (findings stay warnings)
  or are L4 expected-failure cases.
* Examples under ``examples/`` are positive end-to-end fixtures. Negative and
  workflow fixtures live only in ``integration_test/``, never in examples.

Dependencies
------------

* ``test/``: ``rules_testing`` (new analysis tests) next to skylib (existing
  tests), ``aspect_bazel_lib`` (golden update targets). A ``diff_test`` keeps the
  module's own ``.bazelversion`` in sync with the root one.
* ``integration_test/``: ``rules_bazel_integration_test``. The nested Bazel
  version is read from the root ``.bazelversion``; a ``diff_test`` keeps the
  module's own ``.bazelversion`` in sync.
