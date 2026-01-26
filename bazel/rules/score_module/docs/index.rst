SCORE Module Bazel Rules
=========================

This package provides Bazel build rules for defining and building SCORE documentation modules with integrated Sphinx-based HTML generation.

.. contents:: Table of Contents
   :depth: 2
   :local:


Overview
--------

The ``sphinx_module`` package provides two complementary Bazel rules for structuring and documenting software modules:

1. **sphinx_module**: A generic documentation module rule that builds Sphinx-based HTML documentation from RST source files. Suitable for any type of documentation module.

2. **score_component**: A specialized rule for Safety Elements out of Context (SEooC) that enforces documentation structure with standardized artifacts for assumptions of use, requirements, architecture, and safety analysis.

Both rules support **cross-module dependencies** through the ``deps`` attribute, enabling automatic integration of external sphinx-needs references and HTML merging for comprehensive documentation sets.

.. uml:: score_module_overview.puml


Rules and Macros
----------------

sphinx_module
~~~~~~~~~~~~

**File:** ``score_module.bzl``

**Purpose:** Generic rule for building Sphinx-based HTML documentation modules from RST source files with support for dependencies and cross-referencing.

**Usage:**

.. code-block:: python

   sphinx_module(
       name = "my_documentation",
       srcs = glob(["docs/**/*.rst"]),
       index = "docs/index.rst",
       deps = [
           "@score_process//:score_process_module",
           "//other_module:documentation",
       ],
       sphinx = "//bazel/rules/score_module:score_build",
       visibility = ["//visibility:public"]
   )

**Parameters:**

- ``name``: The name of the documentation module
- ``srcs``: List of RST source files for the documentation
- ``index``: Path to the main index.rst file
- ``deps``: Optional list of other ``sphinx_module`` or ``score_component`` targets that this module depends on. Dependencies are automatically integrated for cross-referencing via sphinx-needs and their HTML is merged into the output.
- ``sphinx``: Label to the Sphinx build binary (default: ``//bazel/rules/score_module:score_build``)
- ``config``: Optional custom conf.py file. If not provided, a default configuration is generated.
- ``visibility``: Bazel visibility specification

**Generated Targets:**

- ``<name>``: Main target producing the HTML documentation directory
- ``<name>_needs``: Internal target generating the sphinx-needs JSON file for cross-referencing

**Output:**

- ``<name>/html``: Directory containing the built HTML documentation with integrated dependencies
- ``<name>/needs.json``: Sphinx-needs JSON file for external cross-references

**Build Strategy**

The ``sphinx_module`` rule implements a multi-phase build strategy to ensure proper dependency resolution and documentation integration:

**Phase 1: Generate Needs JSON**

First, the rule builds a ``needs.json`` file for the current module by running Sphinx in a preliminary pass. This JSON file contains all sphinx-needs definitions (requirements, architecture elements, test cases, etc.) from the module's documentation. The needs.json is generated using the ``score_needs`` internal rule.

**Phase 2: Build Dependent Modules**

Before building the main module's HTML, Bazel ensures all modules listed in the ``deps`` attribute are built first. This gives us:

- The ``needs.json`` files from all dependencies for external cross-referencing
- The complete HTML documentation trees from all dependencies for merging

This phase leverages Bazel's dependency graph to parallelize builds where possible.

**Phase 3: Generate Main Module HTML**

With all dependency needs.json files available, Sphinx builds the main module's HTML documentation. During this phase:

- The ``needs_external_needs`` configuration is automatically populated with paths to all dependency needs.json files
- Sphinx resolves ``:need:`` references across module boundaries
- HTML pages are generated in a temporary ``_html`` directory

**Phase 4: Merge HTML Documentation**

Finally, the ``sphinx_html_merge`` tool combines the documentation:

1. Copies the main module's HTML from ``_html/`` to the final ``html/`` output directory
2. For each dependency, copies its ``html/`` directory into the output as a subdirectory
3. Preserves the module hierarchy, enabling navigation between related documentation

The result is a unified documentation tree where users can seamlessly navigate from the main module to any of its dependencies.

**Build Artifacts**

Each successful build produces:

- ``<name>/html/``: Complete merged HTML documentation
- ``<name>/needs.json``: Sphinx-needs export for this module
- ``<name>/_html/``: Intermediate HTML (before merging)


score_component
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

**File:** ``score_module.bzl``

**Purpose:** Specialized macro for defining a Safety Element out of Context (SEooC) module documentation structure and automatic index generation.

**Usage:**

.. code-block:: python

   score_component(
       name = "my_component",
       description = "My safety component providing core functionality.",
       assumptions_of_use = [":my_assumptions_of_use"],
       component_requirements = [":my_component_requirements"],
       architectural_design = [":my_architectural_design"],
       dependability_analysis = [":my_dependability_analysis"],
       checklists = ["docs/safety_checklist.rst"],
       deps = [
           "@score_platform//:score_platform_module",
           "@score_process//:score_process_module",
       ],
       implementations = [":my_lib"],
       tests = [":my_lib_test"],
       visibility = ["//visibility:public"]
   )

**Parameters:**

- ``name``: The name of the safety element module
- ``description``: String containing a high-level description of the SEooC component. This text appears at the beginning of the generated documentation, providing context about what the component does and its purpose. Supports RST formatting.
- ``assumptions_of_use``: List of labels to ``assumptions_of_use`` targets or raw ``.rst``/``.md`` files containing Assumptions of Use documentation
- ``component_requirements``: List of labels to ``component_requirements`` targets or raw ``.rst``/``.md`` files containing component requirements specification
- ``architectural_design``: List of labels to ``architectural_design`` targets or raw ``.rst``/``.md`` files containing architectural design specification
- ``dependability_analysis``: List of labels to ``dependability_analysis`` targets or raw ``.rst``/``.md`` files containing safety analysis documentation (FMEA, DFA, etc.)
- ``checklists``: Optional list of labels to ``.rst`` or ``.md`` files containing safety checklists and verification documents
- ``deps``: Optional list of other ``sphinx_module`` or ``score_component`` targets that this SEooC depends on. Dependencies enable cross-referencing between modules and merge their HTML documentation into the final output.
- ``implementations``: List of labels to implementation targets (cc_library, cc_binary, etc.) that realize the component requirements
- ``tests``: List of labels to test targets (cc_test, py_test, etc.) that verify the implementation against requirements
- ``sphinx``: Label to the Sphinx build binary (default: ``//bazel/rules/score_module:score_build``)
- ``visibility``: Bazel visibility specification

**Generated Targets:**

- ``<name>_seooc_index``: Internal target that generates index.rst and symlinks all artifact files
- ``<name>``: Main SEooC target (internally calls ``sphinx_module``) producing HTML documentation
- ``<name>_needs``: Sphinx-needs JSON file for cross-referencing

**Implementation Details:**

The macro automatically:

- Generates an index.rst file with a toctree referencing all provided artifacts
- Creates symlinks to artifact files (assumptions of use, requirements, architecture, safety analysis) for co-location with the generated index
- Delegates to ``sphinx_module`` for actual Sphinx build and HTML generation
- Integrates dependencies for cross-module referencing and HTML merging

Dependency Management
---------------------

Both ``sphinx_module`` and ``score_component`` support cross-module dependencies through the ``deps`` attribute.
The generated html structure whill look as follows:

.. code-block:: text

   <name>/html/
   ├── index.html                    # Main module documentation
   ├── _static/                      # Sphinx static assets
   ├── dependency_module_1/          # Merged from first dependency
   │   └── index.html
   └── dependency_module_2/          # Merged from second dependency
       └── index.html

This allows seamless navigation between related documentation modules while maintaining independent build targets.

**Example with Dependencies:**

.. code-block:: python

   # Process module (external dependency)
   # @score_process//:score_process_module

   # Platform module (external dependency)
   # @score_platform//:score_platform_module

   # My component that depends on process and platform
   score_component(
       name = "my_component_seooc",
       assumptions_of_use = ["docs/assumptions.rst"],
       component_requirements = ["docs/requirements.rst"],
       architectural_design = ["docs/architecture.rst"],
       dependability_analysis = ["docs/safety.rst"],
       deps = [
           "@score_process//:score_process_module",
           "@score_platform//:score_platform_module",
       ],
   )

Documentation Structure
-----------------------

**For score_component:**

The macro automatically generates an index.rst and organizes files::

   bazel-bin/<name>_seooc_index/
   ├── index.rst                     # Generated toctree
   ├── assumptions_of_use.rst        # Symlinked artifact
   ├── component_requirements.rst    # Symlinked artifact
   ├── architectural_design.rst      # Symlinked artifact
   └── dependability_analysis.rst           # Symlinked artifact

**For sphinx_module:**

User provides the complete source structure::

   docs/
   ├── index.rst                     # User-provided
   ├── section1.rst
   ├── section2.rst
   └── subsection/
       └── details.rst

**Output Structure:**

Both rules produce a standardized output::

   bazel-bin/<name>/
   ├── html/                         # Built HTML documentation
   │   ├── index.html
   │   ├── _static/
   │   ├── <dependency1>/            # Merged dependency HTML
   │   └── <dependency2>/            # Merged dependency HTML
   └── needs.json                    # Sphinx-needs export

Integration with Sphinx
------------------------

The rules provide first-class Sphinx integration:

**Sphinx-Needs Support**

- Automatic configuration of external needs references from dependencies
- Export of needs.json for downstream consumers
- Cross-module traceability using ``:need:`` references

**Sphinx Extensions**

The default configuration includes common SCORE extensions:

- sphinx-needs: Requirements management and traceability
- sphinx_design: Modern UI components
- myst_parser: Markdown support alongside RST

**Custom Configuration**

For ``sphinx_module``, provide a custom conf.py via the ``config`` attribute to override the default Sphinx configuration.


Usage Examples
--------------

Example 1: Generic Documentation Module
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   load("//bazel/rules/score_module:score_module.bzl", "sphinx_module")

   sphinx_module(
       name = "platform_docs",
       srcs = glob(["docs/**/*.rst"]),
       index = "docs/index.rst",
       deps = [
           "@score_process//:score_process_module",
       ],
   )

Build and view:

.. code-block:: bash

   bazel build //:platform_docs
   # Output: bazel-bin/platform_docs/html/

Example 2: Safety Element out of Context
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   load("//bazel/rules/score_module:score_module.bzl",
        "score_component")

   # Implementation
   cc_library(
       name = "kvs_lib",
       srcs = ["kvs.cpp"],
       hdrs = ["kvs.h"],
   )

   # Tests
   cc_test(
       name = "kvs_test",
       srcs = ["kvs_test.cpp"],
       deps = [":kvs_lib"],
   )

   # SEooC with dependencies
   score_component(
       name = "kvs_seooc",
       assumptions_of_use = ["docs/assumptions.rst"],
       component_requirements = ["docs/requirements.rst"],
       architectural_design = ["docs/architecture.rst"],
       dependability_analysis = ["docs/fmea.rst", "docs/dfa.rst"],
       deps = [
           "@score_platform//:score_platform_module",
           "@score_process//:score_process_module",
       ],
       implementations = [":kvs_lib"],
       tests = [":kvs_test"],
   )

Build and view:

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

For complete working examples of all rules and macros, see the test BUILD file:

.. literalinclude:: ../test/BUILD
   :language: python
   :caption: test/BUILD - Complete usage examples
