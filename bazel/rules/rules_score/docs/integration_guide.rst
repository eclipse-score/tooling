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
