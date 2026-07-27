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

Integration Guide
=================

Build Flow
----------

The diagram below shows how input files flow through the Bazel rules to
produce the final outputs.

.. uml:: _assets/seooc_flow.puml
   :align: center
   :alt: SEooC build flow
   :width: 90%

.. _rule-toolchain-configuration:

Toolchain Setup
---------------

``rules_score`` ships a default Sphinx toolchain, registered by ``score_tooling``'s
own ``MODULE.bazel``. **You need nothing to get a working Sphinx build** — plain
sphinx-needs, TRLC and PlantUML documentation builds out of the box for any
module that depends on ``score_tooling``, with no toolchain setup at all.

Registering your own toolchain is only needed when you want **additional**
Sphinx extensions (e.g. Breathe for Doxygen, a custom theme) that the default
doesn't carry. Because Bazel resolves toolchains from the root module first,
a toolchain registered by your own ``MODULE.bazel`` always wins over
``score_tooling``'s default — no special opt-out required.

Adding extensions
~~~~~~~~~~~~~~~~~

Use the ``score_sphinx_toolchain`` macro to extend the default dependency set
instead of reproducing it:

**MODULE.bazel:**

.. code-block:: python

   bazel_dep(name = "score_tooling", version = "1.3.2")

   # Dependency providing your custom Sphinx extension
   bazel_dep(name = "score_docs_as_code", version = "3.0.1", dev_dependency = True)

   register_toolchains("//:my_toolchain")

**BUILD:**

.. code-block:: python

   load("@score_tooling//bazel/rules/rules_score:sphinx_toolchain.bzl", "score_sphinx_toolchain")

   score_sphinx_toolchain(
       name = "my_toolchain",
       extra_deps = [
           "@score_docs_as_code//src/extensions/score_sphinx_bundle",
       ],
   )

This emits ``my_toolchain_binary`` (the Sphinx build binary: the shared
defaults plus ``extra_deps``), ``my_toolchain_info`` (the ``sphinx_toolchain``
target) and ``my_toolchain`` (the ``toolchain()`` itself) — register the last
one.

Diagnosing which toolchain won
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: shell

   bazel cquery --toolchain_resolution_debug='.*rules_score.*' //:my_target

Look for the ``Selected ... toolchain`` line under
``@score_tooling//bazel/rules/rules_score:toolchain_type``.

Replacing the dependency set entirely
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

If your extensions conflict with the shared defaults — e.g. a different pip
hub pinning an incompatible version of a shared package — pass ``deps``
instead of ``extra_deps`` to bypass ``sphinx_base_deps`` entirely and supply
the full list yourself:

.. code-block:: python

   score_sphinx_toolchain(
       name = "my_toolchain",
       conf_template = "//:my_conf.template.py",
       deps = [
           "@score_tooling//bazel/rules/rules_score:sphinx_module_ext",
           "@my_pip_hub//sphinx:pkg",
           "@my_pip_hub//my_custom_extension:pkg",
           # ... full list; sphinx_base_deps is not included in this mode
       ],
   )

``extra_deps`` and ``deps`` are mutually exclusive — passing both fails at
load time.

**score_sphinx_toolchain parameters:**

- ``name`` — name of the emitted ``toolchain()`` target (mandatory)
- ``extra_deps`` — extend mode: extra deps added on top of the shared default set (optional; default: ``[]``)
- ``deps`` — replace mode: exact dep list, bypassing the shared defaults (optional; default: not set)
- ``extra_data`` — extra data files/targets for the Sphinx build binary (optional; default: ``[]``)
- ``conf_template`` — Label to ``conf.py`` template (optional; default: ``@score_tooling//bazel/rules/rules_score:templates/conf.template.py``)
- ``package_collisions`` — forwarded to the generated ``py_binary`` (optional; default: ``"warning"``)
- any other keyword argument (e.g. ``visibility``, ``exec_compatible_with``, ``target_compatible_with``) is forwarded to the ``toolchain()`` target

Assembling a toolchain by hand
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

For cases the macro doesn't fit, use the underlying ``sphinx_toolchain`` rule
directly — it only bundles a Sphinx binary and a ``conf.py`` template into the
provider the rules consume; you own the ``py_binary`` and ``toolchain()``:

.. code-block:: python

   load("@aspect_rules_py//py:defs.bzl", "py_binary")
   load("@score_tooling//bazel/rules/rules_score:sphinx_toolchain.bzl", "sphinx_toolchain")

   py_binary(
       name = "score_build",
       srcs = ["@score_tooling//bazel/rules/rules_score:src/sphinx_wrapper.py"],
       main = "@score_tooling//bazel/rules/rules_score:src/sphinx_wrapper.py",
       visibility = ["//visibility:public"],
       deps = [
           "@score_tooling//bazel/rules/rules_score:sphinx_base_deps",
           "@score_docs_as_code//src/extensions/score_sphinx_bundle",
       ],
   )

   sphinx_toolchain(
       name = "score_sphinx_toolchain",
       sphinx = ":score_build",
   )

   toolchain(
       name = "my_toolchain",
       toolchain = ":score_sphinx_toolchain",
       toolchain_type = "@score_tooling//bazel/rules/rules_score:toolchain_type",
       visibility = ["//visibility:public"],
   )

**sphinx_toolchain parameters:**

- ``sphinx`` — Label to the Sphinx build binary (optional; default: ``@score_tooling//bazel/rules/rules_score:raw_build``)
- ``conf_template`` — Label to ``conf.py`` template (optional; default: ``@score_tooling//bazel/rules/rules_score:templates/conf.template.py``)

The HTML-merge tool used to combine dependency HTML trees is a fixed,
private implementation detail of ``sphinx_module`` — it is not part of
``SphinxInfo`` and cannot be overridden.


Cross-module dependencies
-------------------------

``sphinx_module`` and ``dependable_element`` targets reference each other via
``deps`` to produce merged HTML output:

.. code-block:: text

   <name>/html/
   ├── index.html
   ├── _static/
   ├── dependency1/     ← merged from first dep
   └── dependency2/     ← merged from second dep


Complete Example
----------------

.. code-block:: python

   load("@score_tooling//bazel/rules/rules_score:rules_score.bzl",
        "architectural_design", "assumed_system_requirements",
        "assumptions_of_use", "component", "component_requirements",
        "dependability_analysis", "dependable_element",
        "feature_requirements", "fmea", "unit")

   # Requirements
   assumed_system_requirements(name = "sys_req", srcs = ["docs/sys_req.trlc"])
   feature_requirements(name = "features", srcs = ["docs/features.trlc"],
                        deps = [":sys_req"])
   component_requirements(name = "reqs", srcs = ["docs/reqs.trlc"],
                          deps = [":features"])
   assumptions_of_use(name = "aous", srcs = ["docs/aous.trlc"],
                      requirements = [":features"])

   # Architecture
   architectural_design(name = "arch",
                        static = ["docs/arch.puml"],
                        dynamic = ["docs/sequence.puml"],
                        public_api = ["docs/public_api.puml"])

   # Safety analysis
   fmea(name = "my_fmea", arch_design = ":arch",
        controlmeasures = ["docs/controls.trlc"],
        failuremodes    = ["docs/failures.trlc"],
        root_causes     = ["docs/fta.puml"])
   dependability_analysis(name = "analysis", fmea = [":my_fmea"])

   # Implementation
   cc_library(name = "kvs_lib", srcs = ["kvs.cpp"], hdrs = ["kvs.h"])
   cc_test(name = "kvs_test", srcs = ["kvs_test.cpp"], deps = [":kvs_lib"])

   # Structure
   unit(name = "kvs_unit", unit_design = [":kvs_unit_design"],
        implementation = [":kvs_lib"], tests = [":kvs_test"])
   component(name = "kvs_component", requirements = [":reqs"],
             components = [":kvs_unit"], tests = [])

   # SEooC
   dependable_element(
       name                   = "persistency_kvs",
       integrity_level        = "B",
       assumptions_of_use     = [":aous"],
       requirements           = [":reqs"],
       architectural_design   = [":arch"],
       dependability_analysis = [":analysis"],
       components             = [":kvs_component"],
       tests                  = [],
       deps                   = ["@score_process//:score_process_module"],
   )

Build and test:

.. code-block:: bash

   bazel build //:persistency_kvs
   bazel test  //:persistency_kvs
   # HTML output: bazel-bin/persistency_kvs/html/


Design Rationale
----------------

1. **Two-Tier Architecture** — Generic ``sphinx_module`` for flexibility; specialised artifact rules for safety-critical work products
2. **Dependency Management** — Automatic cross-referencing and HTML merging across modules
3. **Standardisation** — ``dependable_element`` enforces a consistent structure for all safety documentation
4. **Traceability** — Sphinx-needs integration enables bidirectional traceability
5. **Automation** — Index generation, symlinking, and ``conf.py`` management are automatic
6. **Build System Integration** — Bazel ensures reproducible, cacheable documentation builds

Reference implementation: `examples/seooc <https://github.com/eclipse-score/score-tooling/tree/main/bazel/rules/rules_score/examples/seooc>`_ in the score-tooling repository.

---

.. _sphinx-hermetic-tool-setup:

Hermetic Diagram Tools (Graphviz and PlantUML)
----------------------------------------------

The Sphinx HTML action shells out to two diagram tools at **runtime** (inside
Bazel actions): ``dot`` from Graphviz and PlantUML.  Both are hermetic —
i.e.\ no host installation required.  The two tools use different
delivery mechanisms, described below.

Graphviz / ``dot``
~~~~~~~~~~~~~~~~~~

**Source and packaging**

Graphviz now comes directly from the docs runtime sysroot
(``@docs_runtime//:flat``), built with ``rules_distroless`` from
``//third_party/docs_runtime/docs_runtime.yaml``.  The Sphinx action does not
call ``dot`` directly; it uses ``//third_party/docs_runtime:dot`` — an
``exec_in_sysroot`` wrapper that unpacks the sysroot archive and runs
``/usr/bin/dot`` inside it through ``fakechroot``.

**Where the files land (execroot-relative paths)**

.. code-block:: text

   bazel-bin/third_party/docs_runtime/dot          ← GRAPHVIZ_DOT env var
   bazel-bin/third_party/docs_runtime/dot_sysroot/ ← unpacked docs_runtime rootfs
     usr/bin/dot
     usr/lib/graphviz/...
     usr/bin/fakechroot

**Wiring into the Sphinx action**

The Bazel rule sets one variable:

.. list-table::
   :widths: 30 70
   :header-rows: 1

   * - Env var
     - Content
   * - ``GRAPHVIZ_DOT``
     - Path to the ``dot`` binary

The value points to the hermetic wrapper executable.  The wrapper resolves and
executes graphviz from the sysroot itself, so no custom ``LD_LIBRARY_PATH`` /
``GVBINDIR`` wiring is required in the Sphinx action.

**Resolving paths in conf.py**

``GRAPHVIZ_DOT`` is set as an *execroot-relative* path.  Because Sphinx changes
the process working directory during the build, it would break if used as-is.
``conf.template.py`` converts it to a stable absolute path with a single
``os.path.abspath()`` call at **module import time**, when Bazel guarantees the
action's cwd still equals the execroot (before Sphinx performs any
``os.chdir()``).  See :doc:`tooling_architecture` §"Hermetic tool path
resolution" for the full rationale.

PlantUML
~~~~~~~~

**Source and packaging**

PlantUML is fetched from **Maven Central** via ``rules_jvm_external``
(declared in ``MODULE.bazel``).  It is wrapped as a ``java_binary`` at
``//third_party/plantuml:plantuml`` in ``third_party/plantuml/BUILD``.

The ``sphinx_module`` rule passes the target as an action **tool**
(``attr.label(executable = True, cfg = "exec")``), exactly like the hermetic
graphviz dot.  It is not a runfile of the sphinx-build binary.

**Wiring into the Sphinx action**

The Bazel rule sets one variable (mirroring ``GRAPHVIZ_DOT``):

.. list-table::
   :widths: 30 70
   :header-rows: 1

   * - Env var
     - Content
   * - ``PLANTUML_BIN``
     - Execroot-relative path to the ``plantuml`` ``java_binary`` launcher

``PLANTUML_BIN_RLOC`` (the ``short_path`` rlocation key) is also set, but is
used only for diagnostic logging.

**Resolving the path in conf.py**

``PLANTUML_BIN`` is an *execroot-relative* path.  As with ``GRAPHVIZ_DOT``,
``conf.template.py`` converts it to an absolute path with a single
``os.path.abspath()`` call — Bazel guarantees the action's cwd equals the
execroot when ``conf.py`` is imported, before Sphinx performs any
``os.chdir()``.

**Connecting PlantUML to Graphviz**

Once both paths are resolved, ``conf.template.py`` assembles the PlantUML
command:

.. code-block:: python

   plantuml = f"{plantuml_path} -graphvizdot {graphviz_dot}"

The ``-graphvizdot`` flag makes PlantUML use the hermetic ``dot`` binary for
diagram layout instead of its bundled Java port (Smetana).  This ensures the
graphviz version is identical for both ``sphinx.ext.graphviz`` directives and
PlantUML diagrams.  There is no Smetana fallback: the hermetic dot is the
single rendering path.
