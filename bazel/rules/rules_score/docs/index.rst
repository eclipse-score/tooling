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
SCORE Rules for Bazel
=====================

This package provides Bazel build rules for defining and building SCORE documentation modules with integrated Sphinx-based HTML generation.

.. contents:: Table of Contents
   :depth: 2
   :local:


Overview
--------

The ``rules_score`` package provides Bazel rules for structuring and documenting safety-critical software following S-CORE process guidelines:

**Documentation Rule:**

- ``sphinx_module``: Generic rule for building Sphinx HTML documentation with dependency support

**Artifact Rules:**

- ``feature_requirements``: High-level feature specifications
- ``component_requirements``: Component-level requirements
- ``assumptions_of_use``: Safety-relevant operating conditions
- ``architectural_design``: Software architecture documentation
- ``safety_analysis``: Detailed safety analysis (FMEA, FTA)
- ``dependability_analysis``: Comprehensive safety analysis results

**Structural Rules:**

- ``unit``: Smallest testable software element (design + implementation + tests)
- ``component``: Collection of units providing specific functionality
- ``dependable_element``: Complete Safety Element out of Context (SEooC) with full documentation

All rules support cross-module dependencies for automatic sphinx-needs integration and HTML merging.


Toolchain Configuration
-----------------------

The ``sphinx_toolchain`` rule allows you to configure the Sphinx build environment with custom extensions. External modules must define and register their own toolchain to use ``rules_score``.

**Setting Up Toolchain in External Module**

In your MODULE.bazel:

.. code-block:: python

   # Add rules_score dependency
   bazel_dep(name = "score_tooling", version = "1.3.2")
   
   # Add dependencies for custom Sphinx extensions (if needed)
   bazel_dep(name = "score_docs_as_code", version = "3.0.1")
   
   # Register your custom toolchain
   register_toolchains("//:my_toolchain")

In your BUILD file:

.. code-block:: python

   load("@aspect_rules_py//py:defs.bzl", "py_binary")
   load("@score_tooling//bazel/rules/rules_score:sphinx_toolchain.bzl", "sphinx_toolchain")

   # Define Sphinx binary with your required extensions
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

   # Create toolchain instance
   sphinx_toolchain(
       name = "score_sphinx_toolchain",
       sphinx = ":score_build",
   )

   # Register as Bazel toolchain
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

**sphinx_toolchain Parameters:**

- ``sphinx``: Label to Sphinx build binary (mandatory)
- ``conf_template``: Label to conf.py template file (optional, default: ``@score_tooling//bazel/rules/rules_score:templates/conf.template.py``)
- ``html_merge_tool``: Label to HTML merge tool (optional, default: ``@score_tooling//bazel/rules/rules_score:sphinx_html_merge``)


sphinx_module
-------------

Builds Sphinx-based HTML documentation from RST source files with support for dependencies and cross-referencing.

.. code-block:: python

   sphinx_module(
       name = "my_docs",
       srcs = glob(["docs/**/*.rst"]),
       index = "docs/index.rst",
       deps = ["@external_module//:docs"],
       testonly = False,
   )

**Key Parameters:**

- ``srcs``: RST/MD source files
- ``index``: Main index.rst file (mandatory)
- ``deps``: Other sphinx_module or dependable_element targets for cross-referencing
- ``sphinx``: Sphinx build binary (default: ``//bazel/rules/rules_score:score_build``)
- ``testonly``: If true, only testonly targets can depend on this (default: False)
- ``visibility``: Bazel visibility (default: ``["//visibility:public"]``)

**Output:** ``<name>/html/`` with merged dependency documentation

**Note:** Configuration file (``conf.py``) is automatically generated from a template.


Dependency Management
---------------------

Use ``deps`` for cross-module references. HTML is automatically merged:

.. code-block:: text

   <name>/html/
   ├── index.html              # Main documentation
   ├── _static/
   ├── dependency1/            # Merged dependency
   └── dependency2/


Artifact Rules
--------------

Artifact rules define S-CORE process work products. All provide ``SphinxSourcesInfo`` for documentation generation.

**feature_requirements**

.. code-block:: python

   feature_requirements(
       name = "features",
       srcs = ["docs/features.rst"],
   )

**component_requirements**

.. code-block:: python

   component_requirements(
       name = "requirements",
       srcs = ["docs/requirements.rst"],
   )

**assumptions_of_use**

.. code-block:: python

   assumptions_of_use(
       name = "aous",
       srcs = ["docs/assumptions.rst"],
       feature_requirement = [":features"],
       component_requirements = [":requirements"],
   )

**architectural_design**

.. code-block:: python

   architectural_design(
       name = "architecture",
       static = ["docs/static_arch.rst"],
       dynamic = ["docs/dynamic_arch.rst"],
   )

**safety_analysis**

.. code-block:: python

   safety_analysis(
       name = "safety",
       controlmeasures = ["docs/controls.rst"],
       failuremodes = ["docs/failures.rst"],
       fta = ["docs/fta.rst"],
       arch_design = ":architecture",
   )

**dependability_analysis**

.. code-block:: python

   dependability_analysis(
       name = "analysis",
       arch_design = ":architecture",
       dfa = ["docs/dfa.rst"],
       fmea = ["docs/fmea.rst"],
       safety_analysis = [":safety"],
   )


Structural Rules
----------------

**unit**

Define the smallest testable software element.

.. code-block:: python

   unit(
       name = "my_unit",
       unit_design = [":architecture"],
       implementation = ["//src:lib"],
       tests = ["//tests:unit_test"],
       scope = [],
       testonly = True,
   )

**Parameters:**

- ``unit_design``: List of architectural_design targets describing the unit (mandatory)
- ``implementation``: List of implementation targets like cc_library, py_library, etc. (mandatory)
- ``tests``: List of test targets that verify the unit (mandatory)
- ``scope``: Additional targets needed for unit implementation (default: [])
- ``testonly``: If true, only testonly targets can depend on this unit (default: True)
- ``visibility``: Bazel visibility specification

**component**

Define a collection of units.

.. code-block:: python

   component(
       name = "my_component",
       requirements = [":requirements"],
       components = [":my_unit"],
       tests = ["//tests:integration_test"],
       testonly = True,
   )

**Parameters:**

- ``requirements``: List of component_requirements targets (mandatory)
- ``components``: List of unit or component targets that comprise this component (mandatory)
- ``tests``: List of component-level integration test targets (mandatory)
- ``testonly``: If true, only testonly targets can depend on this component (default: True)
- ``visibility``: Bazel visibility specification

**dependable_element**

Define a complete SEooC with automatic documentation generation.

.. code-block:: python

   dependable_element(
       name = "my_seooc",
       description = "My safety-critical component",
       assumptions_of_use = [":aous"],
       requirements = [":requirements"],
       architectural_design = [":architecture"],
       dependability_analysis = [":analysis"],
       components = [":my_component"],
       tests = ["//tests:system_test"],
       checklists = [],
       deps = ["@platform//:platform_module"],
       testonly = True,
   )

**Parameters:**

- ``description``: High-level description of the element (supports RST formatting)
- ``assumptions_of_use``: List of assumptions_of_use targets (mandatory)
- ``requirements``: List of requirements targets (component_requirements, feature_requirements, etc.) (mandatory)
- ``architectural_design``: List of architectural_design targets (mandatory)
- ``dependability_analysis``: List of dependability_analysis targets (mandatory)
- ``components``: List of component and/or unit targets (mandatory)
- ``tests``: List of system-level test targets (mandatory)
- ``checklists``: Optional list of safety checklist files (default: [])
- ``deps``: Optional list of other module targets for cross-referencing (default: [])
- ``testonly``: If true, only testonly targets can depend on this (default: True)
- ``visibility``: Bazel visibility specification

**Generated Targets:**

- ``<name>``: Sphinx module with HTML documentation
- ``<name>_needs``: Sphinx-needs JSON for cross-referencing
- ``<name>_index``: Generated index.rst with artifact structure

**Implementation Details:**

The macro automatically:

- Generates an index.rst file with a toctree referencing all provided artifacts
- Creates symlinks to artifact files (assumptions of use, requirements, architecture, safety analysis) for co-location with the generated index
- Delegates to ``sphinx_module`` for actual Sphinx build and HTML generation
- Integrates dependencies for cross-module referencing and HTML merging


Complete Example
----------------

.. code-block:: python

   load("@score_tooling//bazel/rules/rules_score:rules_score.bzl",
        "architectural_design", "assumptions_of_use",
        "component", "component_requirements",
        "dependability_analysis", "dependable_element",
        "feature_requirements", "safety_analysis", "unit")

   # Artifacts
   feature_requirements(name = "features", srcs = ["docs/features.rst"])
   component_requirements(name = "reqs", srcs = ["docs/reqs.rst"])
   assumptions_of_use(name = "aous", srcs = ["docs/aous.rst"],
                      feature_requirement = [":features"])
   architectural_design(name = "arch", static = ["docs/arch.rst"],
                        dynamic = ["docs/dynamic.rst"])
   safety_analysis(name = "safety", arch_design = ":arch",
                   controlmeasures = ["docs/controls.rst"],
                   failuremodes = ["docs/failures.rst"],
                   fta = ["docs/fta.rst"])
   dependability_analysis(name = "analysis", arch_design = ":arch",
                          dfa = ["docs/dfa.rst"],
                          fmea = ["docs/fmea.rst"],
                          safety_analysis = [":safety"])

   # Implementation
   cc_library(name = "kvs_lib", srcs = ["kvs.cpp"], hdrs = ["kvs.h"])
   cc_test(name = "kvs_test", srcs = ["kvs_test.cpp"], deps = [":kvs_lib"])

   # Structure
   unit(name = "kvs_unit", unit_design = [":arch"],
        implementation = [":kvs_lib"], tests = [":kvs_test"])
   component(name = "kvs_component", requirements = [":reqs"],
             components = [":kvs_unit"], tests = [])

   # SEooC
   dependable_element(
       name = "persistency_kvs",
       description = "Key-Value Store for persistent data storage",
       assumptions_of_use = [":aous"],
       requirements = [":reqs"],
       architectural_design = [":arch"],
       dependability_analysis = [":analysis"],
       components = [":kvs_component"],
       tests = [],
       deps = ["@score_process//:score_process_module"],
   )

Build:

.. code-block:: bash

   bazel build //:persistency_kvs
   # Output: bazel-bin/persistency_kvs/html/
   # Includes merged HTML from dependencies like score_process

Design Rationale
----------------

These rules provide a structured approach to documentation by:

1. **Two-Tier Architecture**: Generic ``sphinx_module`` for flexibility, specialized artifact rules for safety-critical work
2. **Dependency Management**: Automatic cross-referencing and HTML merging across modules
3. **Standardization**: ``dependable_element`` enforces consistent structure for safety documentation
4. **Traceability**: Sphinx-needs integration enables bidirectional traceability
5. **Automation**: Index generation, symlinking, and configuration management are automatic
6. **Build System Integration**: Bazel ensures reproducible, cacheable documentation builds

Reference Implementation
------------------------

See complete examples in the test BUILD file:

.. literalinclude:: ../test/BUILD
   :language: python
   :caption: test/BUILD
