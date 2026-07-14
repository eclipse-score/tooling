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

.. _rule-toolchain-configuration:

Toolchain Setup
---------------

The ``sphinx_toolchain`` rule configures the Sphinx build environment with
custom extensions. External modules must define and register their own toolchain
to use ``rules_score``.

**MODULE.bazel:**

.. code-block:: python

   # Add rules_score dependency
   bazel_dep(name = "score_tooling", version = "1.3.2")

   # Add dependencies for custom Sphinx extensions (if needed)
   bazel_dep(name = "score_docs_as_code", version = "3.0.1")

   # Register your custom toolchain
   register_toolchains("//:my_toolchain")

**BUILD:**

.. code-block:: python

   load("@aspect_rules_py//py:defs.bzl", "py_binary")
   load("@score_tooling//bazel/rules/rules_score:sphinx_toolchain.bzl", "sphinx_toolchain")

   py_binary(
       name = "score_build",
       srcs = ["@score_tooling//bazel/rules/rules_score:src/sphinx_wrapper.py"],
       main = "@score_tooling//bazel/rules/rules_score:src/sphinx_wrapper.py",
       visibility = ["//visibility:public"],
       deps = [
           "@score_tooling//bazel/rules/rules_score:sphinx_module_ext",
           "@score_docs_as_code//src:plantuml_for_python",
           "@score_docs_as_code//src/extensions/score_sphinx_bundle",
           # Add your custom Sphinx extensions here
       ],
   )

   sphinx_toolchain(
       name = "score_sphinx_toolchain",
       sphinx = ":score_build",
   )

   toolchain(
       name = "my_toolchain",
       exec_compatible_with = [
           "@platforms//os:linux",
           "@platforms//cpu:x86_64",
       ],
       target_compatible_with = [
           "@platforms//os:linux",
           "@platforms//cpu:x86_64",
       ],
       toolchain = ":score_sphinx_toolchain",
       toolchain_type = "@score_tooling//bazel/rules/rules_score:toolchain_type",
       visibility = ["//visibility:public"],
   )

**sphinx_toolchain parameters:**

- ``sphinx`` — Label to the Sphinx build binary (mandatory)
- ``conf_template`` — Label to ``conf.py`` template (optional; default: ``@score_tooling//bazel/rules/rules_score:templates/conf.template.py``)
- ``html_merge_tool`` — Label to HTML merge tool (optional; default: ``@score_tooling//bazel/rules/rules_score:sphinx_html_merge``)


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
