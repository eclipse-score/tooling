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


sphinx_module
-------------

Builds Sphinx-based HTML documentation from RST source files with support for dependencies and cross-referencing.

.. code-block:: python

   sphinx_module(
       name = "my_docs",
       srcs = glob(["docs/**/*.rst"]),
       index = "docs/index.rst",
       deps = ["@external_module//:docs"],
   )

**Key Parameters:**

- ``srcs``: RST/MD source files
- ``index``: Main index.rst file
- ``deps``: Other sphinx_module or dependable_element targets for cross-referencing
- ``sphinx``: Sphinx build binary (default: ``//bazel/rules/rules_score:score_build``)

**Output:** ``<name>/html/`` with merged dependency documentation


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
   )

**component**

Define a collection of units.

.. code-block:: python

   component(
       name = "my_component",
       component_requirements = [":requirements"],
       units = [":my_unit"],
       implementation = ["//src:binary"],
       tests = ["//tests:integration_test"],
   )

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
       deps = ["@platform//:platform_module"],
   )

**Generated Targets:**

- ``<name>``: Sphinx module with HTML documentation
- ``<name>_needs``: Sphinx-needs JSON for cross-referencing
- ``<name>_index``: Generated index.rst with artifact structure

       srcs = glob(["docs/**/*.rst"]),
       index = "docs/index.rst",
       deps = ["@external_module//:docs"],
   )

**Key Parameters:**

- ``srcs``: RST/MD source files
- ``index``: Main index.rst file
- ``deps``: Other sphinx_module or dependable_element targets for cross-referencing
- ``sphinx``: Sphinx build binary (default: ``//bazel/rules/rules_score:score_build``)

**Output:** ``<name>/html/`` with merged dependency documentation


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
       feature_requirement = [":features"],
   )

**assumptions_of_use**

.. code-block:: python

   assumptions_of_use(
       name = "aous",
       srcs = ["docs/assumptions.rst"],
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
   )

**component**

Define a collection of units.

.. code-block:: python

   component(
       name = "my_component",
       component_requirements = [":requirements"],
       units = [":my_unit"],
       implementation = ["//src:binary"],
       tests = ["//tests:integration_test"],
   )

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
       deps = ["@platform//:platform_module"],
   )

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

Dependency Management
---------------------

Use ``deps`` for cross-module references. HTML is automatically merged:

.. code-block:: text

   <name>/html/
   ├── index.html              # Main documentation
   ├── _static/
   ├── dependency1/            # Merged dependency
   └── dependency2/


Complete Example
----------------

.. code-block:: python

   load("//bazel/rules/rules_score:rules_score.bzl",
        "architectural_design", "assumptions_of_use",
        "component", "component_requirements",
        "dependability_analysis", "dependable_element",
        "feature_requirements", "safety_analysis", "unit")

   # Artifacts
   feature_requirements(name = "features", srcs = ["docs/features.rst"])
   component_requirements(name = "reqs", srcs = ["docs/reqs.rst"],
                          feature_requirement = [":features"])
   assumptions_of_use(name = "aous", srcs = ["docs/aous.rst"])
   architectural_design(name = "arch", static = ["docs/arch.rst"],
                        dynamic = ["docs/dynamic.rst"])
   safety_analysis(name = "safety", arch_design = ":arch")
   dependability_analysis(name = "analysis", arch_design = ":arch",
                          dfa = ["docs/dfa.rst"],
                          safety_analysis = [":safety"])

   # Implementation
   cc_library(name = "kvs_lib", srcs = ["kvs.cpp"], hdrs = ["kvs.h"])
   cc_test(name = "kvs_test", srcs = ["kvs_test.cpp"], deps = [":kvs_lib"])

   # Structure
   unit(name = "kvs_unit", unit_design = [":arch"],
        implementation = [":kvs_lib"], tests = [":kvs_test"])
   component(name = "kvs_component", requirements = [":reqs"],
             units = [":kvs_unit"], implementation = [":kvs_lib"], tests = [])

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

   # Implementation
   cc_library(name = "kvs_lib", srcs = ["kvs.cpp"], hdrs = ["kvs.h"])
   cc_test(name = "kvs_test", srcs = ["kvs_test.cpp"], deps = [":kvs_lib"])

   # Structure
   unit(name = "kvs_unit", unit_design = [":arch"],
        implementation = [":kvs_lib"], tests = [":kvs_test"])
   component(name = "kvs_component", component_requirements = [":reqs"],
             units = [":kvs_unit"], implementation = [":kvs_lib"], tests = [])

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

   bazel build //:kvs_seooc
   # Output: bazel-bin/kvs_seooc/html/
   # Includes merged HTML from score_platform and score_process modules

Design Rationale
----------------

These rules provide a structured approach to documentation by:

1. **Two-Tier Architecture**: Generic ``sphinx_module`` for flexibility, specialized ``score_component`` for safety-critical work
2. **Dependency Management**: Automatic cross-referencing and HTML merging across modules
3. **Standardization**: SEooC enforces consistent structure for safety documentation
4. **Traceability**: Sphinx-needs integration enables bidirectional traceability
5. **Automation**: Index generation, symlinking, and configuration management are automatic
6. **Build System Integration**: Bazel ensures reproducible, cacheable documentation builds

Reference Implementation
------------------------

See complete examples in the test BUILD file:

.. literalinclude:: ../test/BUILD
   :language: python
   :caption: test/BUILD
