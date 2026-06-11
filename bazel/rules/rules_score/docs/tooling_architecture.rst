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

Architecture
============

This page is the developer-facing map of ``rules_score``: how the Bazel
rules/macros are wired together, **which tool each rule invokes**, how the
tools interrelate, and what every tool does. It complements the user-facing
:doc:`overview` (rule catalogue) and :doc:`integration_guide` (how to consume
the rules).

Three layers
------------

``rules_score`` is organised in three layers. A user only touches layer 1; the
build wires layers 2 and 3 automatically.

#. **Macros / rules** (Starlark, ``private/*.bzl``) — the public work-product
   declarations (``feature_requirements``, ``architectural_design``, ``fmea``,
   ``unit``, ``component``, ``dependability_analysis``, ``dependable_element``,
   …). Each one declares actions and emits **providers**.
#. **Providers** (``providers.bzl``) — the typed contracts that carry data
   between rules (e.g. ``ArchitecturalDesignInfo``, ``AnalysisInfo``,
   ``SphinxSourcesInfo``). The provider graph is the "API" between rules; see
   :doc:`overview` for the provider-flow diagram.
#. **Tools** — the executables each action runs. Some are vendored third-party
   tools (TRLC, Lobster, the PlantUML parser, Sphinx); some are local helpers
   under ``src/`` (``rst_to_trlc.py``, ``safety_analysis_tools.py``,
   ``sphinx_html_merge.py``).

Rule → tool invocation map
--------------------------

The figure below shows, for each rule, the tool(s) it drives during
``bazel build`` / ``bazel test``. Green boxes are local helpers shipped in this
repository; blue boxes are external tools.

.. uml:: _assets/tooling_chain.puml
   :align: center
   :alt: Which tool each rules_score rule invokes
   :width: 100%

Tool inventory
--------------

Every tool below is also registered as a ``ToolQualification.Tool`` record in
``trlc/config/tools.trlc`` so it can be referenced from use cases, potential
errors and tool requirements (see :doc:`tool_qualification`). Per-tool READMEs
are rendered under :doc:`tool_reference/index`.

.. list-table::
   :header-rows: 1
   :widths: 16 22 22 40

   * - Tool
     - Binary / label
     - Invoked by
     - Function
   * - **Bazel**
     - build system
     - all rules
     - Orchestrates the action graph, propagates providers, enforces hermetic
       builds, and runs the analysis-time **certified-scope** and
       **integrity-level** safety checks in ``dependable_element``.
   * - **TRLC**
     - ``@trlc//tools/trlc_rst:trlc_rst`` + TRLC parser;
       ``trlc_requirements_test``
     - ``feature_requirements``, ``component_requirements``,
       ``assumed_system_requirements``, ``fmea``
     - Parses and type-checks requirement / FMEA records against the ``.rsl``
       metamodel, then renders them to ``.rst`` (requirements) or ``.inc``
       (FMEA sections) for Sphinx.
   * - **rst_to_trlc**
     - ``src/rst_to_trlc.py`` (local)
     - ``score_requirements_rule`` macro
     - Converts RST requirement directives (``feat_req``, ``comp_req``, …) into
       ``.trlc`` records so requirements can be authored in either RST or TRLC.
   * - **PlantUML Parser**
     - ``@score_tooling//plantuml/parser:parser`` (Rust) + ``:linker``
     - ``architectural_design``, ``unit_design``
     - Parses ``.puml`` diagrams into a FlatBuffers AST (``.fbs.bin``, one
       ``root_type`` per diagram kind) and extracts interface ``.lobster``
       items. The **linker** merges the FlatBuffers into ``plantuml_links.json``
       for the ``clickable_plantuml`` Sphinx extension. Rejects syntactically
       invalid diagrams with a non-zero exit code.
   * - **safety_analysis_tools**
     - ``//bazel/rules/rules_score:safety_analysis_tools``
       (``src/safety_analysis_tools.py``, local)
     - ``fmea``
     - Inlines ``fta_metamodel.puml`` into root-cause FTA diagrams (making them
       hermetic) and extracts ``$TopEvent`` / ``$BasicEvent`` calls into
       ``root_causes.lobster`` in ``lobster-act-trace`` format.
   * - **Lobster**
     - ``@lobster//`` : ``lobster-trlc``, ``lobster-report``,
       ``lobster-ci-report``, ``lobster-html-report``, ``gtest_report``,
       ``lobster-rst-report``
     - ``*_requirements``, ``fmea``, ``unit``, ``dependability_analysis``,
       ``dependable_element``
     - The traceability backbone. ``lobster-trlc`` extracts ``.lobster`` items
       from TRLC; ``gtest_report`` turns test results into ``.lobster``;
       ``lobster-report`` aggregates against a generated config;
       ``lobster-ci-report`` is the **pass/fail gate**; the ``html``/``rst``
       variants render the report into the documentation.
   * - **Architecture Verifier**
     - ``//validation/core:validation_cli``
     - ``dependable_element`` (``_index`` rule)
     - Compares the *current* architecture (components/units collected from the
       implementation tree via an aspect, serialised to ``architecture.json``)
       against the *expected* architecture (static/dynamic ``.fbs.bin`` from
       ``architectural_design``). Fails the build on a mismatch.
   * - **Sphinx (Docs)**
     - ``score_build`` (``src/sphinx_wrapper.py``),
       ``html_merge_tool`` (``src/sphinx_html_merge.py``),
       ``sphinx_module_ext`` / ``bazel_sphinx_needs``
     - ``sphinx_module``, ``dependable_element``
     - Two-phase documentation build: phase 1 emits ``needs.json`` for
       sphinx-needs cross-referencing, phase 2 builds HTML and merges
       dependency modules into one site.
   * - **Lobster Bazel**
     - ``//lobster_bazel:lobster_linker`` (``parse_source_files.py``)
     - ``rules_score_impl`` (tool-qualification chain)
     - Scans source files (C/C++, Rust, Python, Starlark, TRLC) for
       single-line tracing tags (``lobster-trace`` / ``req-traceability``) and
       emits a ``.lobster`` file. This is what closes the qualification chain
       from **Tool Requirements down to the implementing source code**.
   * - **Manual Analysis**
     - ``//manual_analysis`` (``manual_analysis`` macro)
     - standalone qualification activity
     - Captures human review verdicts against an analysis spec, locks them to a
       committed lock/results file, and emits the verdict as ``.lobster`` so
       manual judgements participate in the same traceability report as the
       automated checks.

.. note::

   ``tools.trlc`` also registers placeholder tools that are not yet wired into
   any action: ``PlantumlLinter``, ``PlantumlFormatter``, ``LibClang`` (planned
   C/C++ design extraction), ``BazelMetamodel`` (the FlatBuffers architecture
   schema, currently embedded in the parser/verifier), and ``AIChecker``. They
   exist so use cases and tool requirements can already reference them.

How the tools interrelate — the traceability data flow
------------------------------------------------------

The unifying idea is that **every artefact is reduced to a ``.lobster`` file**,
and all ``.lobster`` files are aggregated and gated by Lobster. Different tools
feed that pipeline:

.. uml:: _assets/traceability_dataflow.puml
   :align: center
   :alt: Traceability data flow from artefacts to the lobster gate
   :width: 100%

* **Requirements** (``.trlc``) → ``lobster-trlc`` → ``requirements.lobster``.
* **Public API diagrams** (``public_api.puml``) → PlantUML parser →
  ``public_api.lobster`` (enables failure-mode-to-interface tracing).
* **FMEA** (``failuremodes.trlc`` / ``controlmeasures.trlc``) → ``lobster-trlc``;
  **FTA** (``fta.puml``) → ``safety_analysis_tools`` → ``root_causes.lobster``.
* **Unit tests** (gtest) → ``gtest_report`` → ``<unit>.lobster``.
