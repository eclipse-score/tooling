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
the process working directory during the build, these paths would break if
used as-is.  ``conf.template.py`` therefore:

1. Captures ``_EXECROOT = Path.cwd()`` at **module import time** (cwd is still
   the execroot at that point).
2. Calls ``_resolve_execroot_path()`` on the value to prepend ``_EXECROOT``
   and produce absolute paths.

PlantUML
~~~~~~~~

**Source and packaging**

PlantUML is fetched from **Maven Central** via ``rules_jvm_external``
(declared in ``MODULE.bazel``).  It is wrapped as a ``java_binary`` at
``//tools/sphinx:plantuml`` in ``tools/sphinx/BUILD``.

The PlantUML target is added to the sphinx-build binary (``raw_build``) as a
``data`` dependency, making it a **runfile** of that binary — not an
independent action input.

**Where the file lands (runfiles-relative path)**

.. code-block:: text

   {sphinx_build_binary}.runfiles/
     {repo_name}/tools/sphinx/plantuml   ← wrapper script (absolute path via Runfiles API)

The ``{repo_name}`` prefix depends on the Bzlmod configuration:

- ``_main`` — when score_tooling is the root module (e.g.\ building within
  the score-tooling repo itself)
- ``score_tooling`` / ``score_tooling+`` / ``score_tooling~`` — when
  score_tooling is an external dependency of another project

**Discovering the binary in conf.py**

Because the repo name varies, ``conf.template.py`` uses two-stage discovery:

1. **Manifest scan (primary):** Read ``RUNFILES_MANIFEST_FILE`` and search
   for any entry whose runfiles path ends in ``/tools/sphinx/plantuml``.  This
   requires no knowledge of the repo name prefix.
2. **Runfiles API fallback:** If no manifest file is available (directory-based
   runfiles trees on some platforms), fall back to
   ``Runfiles.Create().Rlocation()`` with a list of known repo-name candidates
   (``_main``, ``score_tooling``, ``score_tooling+``, …).

The Runfiles API returns an **absolute path** directly, so no
``_resolve_execroot_path()`` is required for PlantUML.

**Connecting PlantUML to Graphviz**

Once both paths are resolved, ``conf.template.py`` assembles the PlantUML
command:

.. code-block:: python

   plantuml = f"{plantuml_path} -graphvizdot {graphviz_dot}"

The ``-graphvizdot`` flag makes PlantUML use the hermetic ``dot`` binary for
diagram layout instead of its bundled Java port (Smetana).  This ensures that
the graphviz version is identical for both ``sphinx.ext.graphviz`` directives
and PlantUML diagrams.
