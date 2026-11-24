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
     - ``bazel/rules/rules_score/src/``, ``plantuml/parser/``
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
The legacy ``test/BUILD`` and the tool tests next to it (``test/test_*.py``) stay
in place; they move into the level directories rule by rule. Each level
directory holds one smoke test (``label_path_test`` at L1, ``rst_to_trlc`` at
L2 and L3) as the template for new tests.

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
     - Label keys, scope calculation, config building
     - Providers, actions
   * - L2
     - Providers, actions, ``FileWrite`` content, analysis errors
     - Action arguments (maturity flags), ``validation_inputs.json`` content,
       ``expect_failure`` messages
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
outputs (for example ``architecture.json`` and ``*_architecture.lobster``). They
are not used for pass/fail logs, HTML, or content already readable at L2
(``validation_inputs.json``). Goldens are declared with ``write_source_files``
(``aspect_bazel_lib``): ``bazel run`` of the ``<name>`` target refreshes the
golden, ``<name>_test`` diffs it.

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
