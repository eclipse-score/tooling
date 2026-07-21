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
   under ``src/`` (``rst_to_trlc.py``, ``fmea_assembler.py``,
   ``sphinx_html_merge.py``). The FMEA fault-tree processing lives in the Rust
   ``puml_cli`` (FTA mode, backed by the ``puml_fta`` crate).

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
       metamodel and renders them to ``.rst``.  ``trlc_rst`` also ships a
       reusable ``TRLCRST`` library that ``fmea_assembler`` links directly to
       build the FMEA page from a single in-process parse (no per-record
       ``.inc`` files).
   * - **rst_to_trlc**
     - ``src/rst_to_trlc.py`` (local)
     - ``score_requirements_rule`` macro
     - Converts RST requirement directives (``feat_req``, ``comp_req``, …) into
       ``.trlc`` records so requirements can be authored in either RST or TRLC.
   * - **PlantUML Parser**
     - ``@score_tooling//plantuml/parser:parser`` (Rust)
     - ``architectural_design``, ``unit_design``
     - Parses ``.puml`` diagrams into a FlatBuffers AST (``.fbs.bin``, one
       ``root_type`` per diagram kind), extracts interface ``.lobster`` items,
       and emits ``.idmap.json`` sidecars recording the *defines/references*
       roles of each element.  The ``clickable_plantuml`` Sphinx extension reads
       these sidecars to resolve cross-diagram links without a separate linker
       step.  Rejects syntactically invalid diagrams with a non-zero exit code.
   * - **puml_cli (FTA mode)**
     - ``//plantuml/parser/puml_cli`` ``--fta-output-dir`` (Rust; FTA model in
       the ``puml_fta`` crate)
     - ``fmea``
     - Analysis only: parses the ``$TopEvent`` / ``$BasicEvent`` / gate macro
       calls of each root-cause FTA diagram into
       two outputs: ``root_causes.lobster`` (``lobster-act-trace``) and
       ``fta_chains.json`` (the ordered per-failure-mode chains).  The authored
       diagram keeps its ``!include fta_metamodel.puml``; the metamodel ships in
       the docs toolchain runfiles and is put on PlantUML's global include path
       (``-Dplantuml.include.path``) so the include resolves at render even under
       sphinxcontrib-plantuml's ``-pipe`` mode.  Unrooted basic events and
       malformed TRLC aliases are reported as build warnings rather than silently
       dropped.
   * - **fmea_assembler**
     - ``//bazel/rules/rules_score:fmea_assembler``
       (``src/fmea_assembler.py``, local; links the ``TRLCRST`` library)
     - ``fmea``
     - Assembles the failure-mode-centric ``fmea.rst`` from ``fta_chains.json``
       plus the FailureMode / ControlMeasure records in one in-process TRLC
       parse: an overview table, one section per failure mode (detail + inline
       fault tree + that chain's control measures), and trailing "Unlinked"
       sections so nothing is dropped.
   * - **safety_analysis_tools**
     - ``//bazel/rules/rules_score:safety_analysis_tools``
       (``src/safety_analysis_tools.py``, local)
     - ``fmea``
     - Assembles the failure-mode-centric ``fmea.rst`` from ``fta_chains.json``
       plus the FailureMode / ControlMeasure records in one in-process TRLC
       parse: an overview table, one section per failure mode (detail + inline
       fault tree + that chain's control measures), and trailing "Unlinked"
       sections so nothing is dropped.
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
       ``sphinx_module_ext``,
       ``trlc`` Sphinx extension (``@trlc``)
     - ``sphinx_module``, ``dependable_element``
     - Two-phase documentation build: **phase 1** (``<name>_needs`` target)
       runs Sphinx with ``--builder needs`` to emit ``needs.json`` containing
       any native ``sphinx-needs`` (``.. need::``) directives found in the
       sources.  **Phase 2** (``<name>`` target) runs Sphinx with
       ``--builder html``, resolving ``trlc`` ``.. requirement:definition::``
       cross-references within the relocated source tree and consuming
       ``needs.json`` files of all ``deps`` via ``needs_external_needs_json``
       for cross-module ``sphinx-needs`` links.
       ``src/sphinx_html_merge.py`` then merges dependency HTML directories
       into the final output site.
       See :ref:`two-phase-sphinx-build` for details.
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
  **FTA** (``fta.puml``) → ``puml_cli`` (FTA mode) → ``root_causes.lobster``.
* **Unit tests** (gtest) → ``gtest_report`` → ``<unit>.lobster``.

.. _two-phase-sphinx-build:

Two-phase Sphinx build
----------------------

Every ``sphinx_module`` call expands into **two** Bazel targets that run
sequentially:

.. code-block:: text

   <name>_needs  (phase 1 — needs builder)
   <name>        (phase 2 — HTML builder)

Phase 1 — ``<name>_needs``
~~~~~~~~~~~~~~~~~~~~~~~~~~

Sphinx is invoked with ``--builder needs`` against the **static docs/ source
tree** (only ``srcs`` files — the checked-in ``.rst``/``.md`` files plus the
generated ``trlc_rst`` outputs that are listed as label targets in ``srcs``).
Generated/external files from ``renamed_srcs`` and ``docs_library_deps`` are
**not** included; their toctree entries produce ``toc.not_readable`` warnings
that are suppressed in ``conf.template.py`` (see below for why this is safe).

The needs builder scans every document for ``.. need::`` directives
(``sphinx-needs`` native format) and writes them to a ``needs.json`` file.

The ``toc.not_readable`` suppression in ``conf.template.py`` is safe for the
HTML phase because that phase relocates every file into a staging directory, so
it never encounters an unresolvable toctree entry.

The resulting ``needs.json`` (empty for modules whose requirements are authored
in TRLC rather than native ``sphinx-needs`` format) is wrapped in a
``SphinxNeedsInfo`` provider and propagated transitively so that every
downstream module can consume it.

Phase 2 — ``<name>``
~~~~~~~~~~~~~~~~~~~~

Sphinx is invoked with ``--builder html`` against a **relocated copy** of all
source files (``srcs``, ``renamed_srcs``, ``docs_library_deps``) symlinked into
a unified staging directory under ``bazel-bin/``.

This is also where ``.. requirement:definition::`` directives (from the ``trlc``
Sphinx extension) are processed and cross-references resolved.  The raw
requirement records come from ``.trlc`` source files compiled by the
``trlc_rst`` Bazel rule into ``.rst`` files that contain the directives.  The
chain is:

.. code-block:: text

   *.trlc
     └─ trlc_rst  (Bazel rule, @trlc)
          └─ requirements_rst.rst  (.. requirement:definition:: <ID> ...)
               └─ Sphinx HTML builder  (resolves {requirement:downstream-ref})

Before the HTML build starts, ``sphinx_module_ext.py`` reads the aggregated
``needs_external_needs.json``
(written by the Bazel rule from all incoming ``SphinxNeedsInfo`` providers) and
populates the ``needs_external_needs`` Sphinx configuration key. This tells
``sphinx-needs`` where to find the ``needs.json`` of each dependency and what
base URL to use for generated hyperlinks, so a ``{requirement:downstream-ref}``
role in a spec file can link directly to the requirement definition page in the
dependency's HTML.

After the HTML build, ``src/sphinx_html_merge.py`` copies each dependency's
output directory into ``<name>/html/<dep-name>/`` so the final site is
self-contained.

.. code-block:: text

   deps[*].needs.json  ──► needs_external_needs.json
                                  │
   sources (all relocated) ───────┤
                                  ▼
                           Sphinx HTML builder
                                  │
                           <name>/_html/  ──► sphinx_html_merge
                                                     │
                                              <name>/html/
                                              ├── index.html
                                              ├── dep1/     ← merged
                                              └── dep2/     ← merged

.. _safety-analysis-doc-pipeline:

Safety analysis document pipeline
----------------------------------

The component diagram below shows how the FMEA **input artifacts** — authored
``.trlc`` records and ``fta_*.puml`` diagrams plus the tooling defaults
(``ScoreReq`` ``.rsl`` spec, ``fta_metamodel.puml``, ``fmea.template.rst`` and
the lobster configs) — flow through the three in-process tool actions of the
``fmea`` rule into the generated files, the providers, and finally the Sphinx
staging tree.  Blue boxes are authored sources, light-blue are tooling defaults,
green components are the tool actions, orange boxes are generated files, yellow
boxes are the provider payloads, and the purple box is the staging directory
consumed by Sphinx.

.. uml:: _assets/safety_analysis_doc_pipeline.puml
   :align: center
   :alt: Safety analysis document pipeline
   :width: 100%

The ``fmea`` rule drives three actions, all reading the input artifacts above:

#. **puml_cli (FTA mode)** parses each ``fta_*.puml`` directly (no rewriting)
   and writes ``root_causes.lobster`` and ``fta_chains.json`` (the ordered
   per-failure-mode chains).  The diagrams keep their ``!include
   fta_metamodel.puml``; the metamodel is on PlantUML's global include path
   (shipped in the docs toolchain runfiles), so it resolves at render time.
#. **fmea_assembler** consumes ``fta_chains.json`` and parses the FailureMode /
   ControlMeasure ``.trlc`` records (with the ``.rsl`` spec for import
   resolution) in a single in-process ``TRLCRST`` pass, expanding
   ``fmea.template.rst`` into ``fmea.rst``.
#. **lobster-trlc** (run twice) turns the FailureMode and ControlMeasure records
   into ``failuremodes.lobster`` / ``controlmeasures.lobster`` for the
   traceability report.

``SphinxSourcesInfo`` carries three depsets:

- **srcs** — files that become top-level toctree entries in the enclosing
  document section.  ``fmea`` emits exactly one: ``fmea.rst``.
- **deps** — all files that must be present in the staging directory; for
  ``fmea`` this is just ``fmea.rst``, because the page is self-contained
  (failure modes and control measures are rendered inline, not pulled in via
  ``.. include::``).
- **aux_srcs** — files to symlink alongside ``srcs``/``deps`` but **not** added
  to any toctree.  ``fmea`` uses this for the authored ``fta_*.puml`` diagrams,
  which ``fmea.rst`` references inline via ``.. uml::`` and which must therefore
  sit beside it in the staging tree without being indexed as documents.  (The
  metamodel is not staged here — it resolves via PlantUML's global include
  path.)

The lobster outputs travel separately on ``AnalysisInfo.lobster_files``
(``failuremodes.lobster``, ``controlmeasures.lobster``, ``root_causes.lobster``)
into the ``dependability_analysis`` traceability report.

.. _hermetic-tool-path-resolution:

Hermetic tool path resolution
------------------------------

Background: Bazel action environment
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Paths available at **analysis time** (Starlark ``ctx.executable.foo.path``,
``ctx.executable.foo.short_path``) are always relative to the *execroot* —
the per-action working directory Bazel creates under the output base.  Two
variants exist:

- ``file.path`` — ``bazel-out/<config-hash>/bin/third_party/docs_runtime/dot``.
  Contains the exec-configuration hash; valid only at action run-time as a
  path relative to ``cwd``.
- ``file.short_path`` — ``third_party/docs_runtime/dot`` (or
  ``../external_repo/…``).  Hash-free; stable across rebuilds; the canonical
  **rlocation key** after stripping a leading ``../``.

What the rule passes
~~~~~~~~~~~~~~~~~~~~~

For each tool ``sphinx_module.bzl`` computes both variants and injects them as
environment variables:

.. code-block:: text

   PLANTUML_BIN      = ctx.executable._plantuml.path      (execroot-relative)
   PLANTUML_BIN_RLOC = ctx.executable._plantuml.short_path (rlocation key)
   GRAPHVIZ_DOT      = ctx.executable._graphviz.path      (execroot-relative)
   GRAPHVIZ_DOT_RLOC = ctx.executable._graphviz.short_path (rlocation key)

The rlocation keys (``*_RLOC``) are computed once at analysis time:

.. code-block:: python

   _gv_short = ctx.executable._graphviz.short_path
   _graphviz_rloc = (
       _gv_short[3:]                                    # strip "../"
       if _gv_short.startswith("../")
       else ctx.workspace_name + "/" + _gv_short
   )

This matches the Bazel runfiles convention used internally by the
``exec_in_sysroot`` wrapper itself (``rlocation
'<workspace>/third_party/docs_runtime/dot'``).

How conf.template.py resolves the paths
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``conf.py`` is loaded during **Sphinx initialisation**, before Sphinx performs
any ``os.chdir()``.  Bazel guarantees that the action's ``cwd`` equals the
execroot at process start.  Therefore a single ``os.path.abspath()`` call
converts the execroot-relative ``*_BIN`` / ``*_DOT`` value to a stable
absolute path for the entire action lifetime:

.. code-block:: python

   plantuml_path = os.path.abspath(os.environ["PLANTUML_BIN"])
   graphviz_dot  = os.path.abspath(os.environ["GRAPHVIZ_DOT"])


Why rlocation alone cannot resolve the binary
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

A natural question is: why not call
``python.runfiles.Runfiles.Create().Rlocation(graphviz_rloc_key)`` directly and
skip the ``abspath`` step?

Two reasons make this impractical:

1. **Wrong ``RUNFILES_DIR``.**  The Sphinx Python process inherits
   ``RUNFILES_DIR`` pointing to the *sphinx tool's* own runfiles tree (set by
   the ``rules_python`` ``py_binary`` launcher).  The Graphviz and PlantUML
   tools are in ``tools=`` of the action, which makes their files available in
   the sandbox but does **not** merge them into the sphinx binary's runfiles.
   Calling ``Runfiles.Create()`` without an override therefore searches the
   wrong tree and returns ``None`` for both tool keys.

2. **The symlink-path problem.**  As a workaround one could construct
   ``Runfiles.Create({"RUNFILES_DIR": abspath_dot + ".runfiles"})`` (the
   companion runfiles directory Bazel creates for every executable).  The dot
   wrapper *is* a member of its own runfiles tree (``exec_in_sysroot`` adds
   the generated script to ``ctx.runfiles(files=[out, …])`` so the smoke test
   can resolve it via ``rlocation``).  However, ``Runfiles.Rlocation()``
   returns the path **inside the symlink forest**, not the real binary path.
   Passing that symlink path to a subprocess means ``$0`` is the symlink, so
   ``$0.runfiles/`` does not exist and the wrapper's runfiles bootstrap falls
   back to ``RUNFILES_DIR`` — which still points to the sphinx binary's
   runfiles.  The wrapper fails to find ``SYSROOT_DIR`` and ``FAKECHROOT_WRAPPER``
   and exits with an error.

The ``os.path.abspath()`` approach avoids both issues: it yields the real
binary path (not a symlink in a runfiles forest), so ``$0.runfiles/``
bootstrapping in the wrapper works correctly.

The exec_in_sysroot wrapper's own runfiles bootstrap
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The generated wrapper produced by ``exec_in_sysroot`` is a POSIX-``sh`` script
that contains an inline runfiles initialisation block (no ``bash`` and no
``runfiles.bash`` dependency):

.. code-block:: sh

   if [ ! -d "${RUNFILES_DIR:-/dev/null}" ] && \
      [ ! -f "${RUNFILES_MANIFEST_FILE:-/dev/null}" ]; then
     if [ -f "$0.runfiles_manifest" ]; then
       RUNFILES_MANIFEST_FILE="$0.runfiles_manifest"; export RUNFILES_MANIFEST_FILE
     elif [ -f "$0.runfiles/MANIFEST" ]; then
       RUNFILES_MANIFEST_FILE="$0.runfiles/MANIFEST"; export RUNFILES_MANIFEST_FILE
     elif [ -d "$0.runfiles" ]; then
       RUNFILES_DIR="$0.runfiles"; export RUNFILES_DIR
     fi
   fi

Because ``graphviz_dot`` is the **absolute** path to the real binary (not a
symlink), ``$0`` equals that absolute path, and ``$0.runfiles/`` is the actual
companion runfiles directory Bazel created for the binary.  The block resolves
``RUNFILES_DIR`` (or ``RUNFILES_MANIFEST_FILE``) from ``$0.runfiles/``, and a
small inline ``rlocation`` helper then looks up ``SYSROOT_DIR`` and the
fakechroot wrapper — regardless of what ``RUNFILES_DIR`` the parent process
(Sphinx/Python) has set in its environment.

PlantUML and the hermetic dot
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``sphinxcontrib.plantuml`` invokes PlantUML via
``shlex.split(app.config.plantuml)``.  The ``plantuml`` config value always
points at the hermetic dot (there is no fallback):

.. code-block:: text

   <abs_plantuml_path> -graphvizdot <abs_graphviz_dot>

This tells PlantUML to use the hermetic dot for its internal layout calls
(PlantUML generates a ``.dot`` intermediate for class/component/sequence
diagrams and hands it to graphviz for layout).
