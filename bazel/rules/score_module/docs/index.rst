SCORE Module Bazel Rules
=========================

This directory contains Bazel rules for defining and building SCORE safety modules.

.. contents:: Table of Contents
   :depth: 2
   :local:


Overview
--------

The ``score_module`` package provides Bazel build rules to structure,
validate, and document safety-critical software modules. These rules
integrate with Sphinx documentation generation to produce comprehensive
safety documentation.

.. uml::

   @startuml
    [SEooC] as SEooC
    [bazel module] as bzlmod
    Artifact "Assumptions of Use" as AoU
    Artifact "(Assumed) Component Requirements" as CR <<document>>
    Artifact "Architecture Design" as AD <<document>>
    Artifact "Safety Analysis" as SA <<document>>
    Card "Implementation" as Impl <<target>>
    Card "Testsuite" as Test <<target>>



    bzlmod "1" *-- "*" SEooC : contains
    SEooC ..> SEooC : depends on
    SEooC "1" *-- "1" AoU : has
    SEooC "1" *-- "1" CR : has
    SEooC "1" *-- "1" AD : has
    SEooC "1" *-- "1" Impl : has
    SEooC "1" *-- "1" Test : has
    SEooC "1" *-- "1" SA : has

    note right of bzlmod
        A score_module can contain
        one or more Safety Elements
        out of Context (SEooC)
    end note

   @enduml





Rules and Macros
----------------

safety_element_out_of_context
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

**File:** ``score_module.bzl``

**Purpose:** Main macro for defining a Safety Element out of Context
(SEooC) module with integrated documentation generation following ISO 26262
standards.

**Usage:**

.. code-block:: python

   safety_element_out_of_context(
       name = "my_module",
       assumptions_of_use = ":assumptions",
       component_requirements = ":requirements",
       architectural_design = ":architecture",
       safety_analysis = ":safety_analysis",
       implementations = [":my_lib", ":my_component"],
       tests = [":my_lib_test", ":my_integration_test"],
       visibility = ["//visibility:public"]
   )

**Parameters:**

- ``name``: The name of the safety element module. Used as the base name
  for all generated targets.
- ``assumptions_of_use``: Label to a ``.rst`` or ``.md`` file containing the
  Assumptions of Use, which define the safety-relevant operating conditions
  and constraints for the SEooC as required by ISO 26262-10 clause 5.4.4.
- ``component_requirements``: Label to a ``.rst`` or ``.md`` file containing
  the component requirements specification, defining functional and safety
  requirements.
- ``architectural_design``: Label to a ``.rst`` or ``.md`` file containing
  the architectural design specification, describing the software architecture
  and design decisions as required by ISO 26262-6 clause 7.
- ``safety_analysis``: Label to a ``.rst`` or ``.md`` file containing the
  safety analysis, including FMEA, FMEDA, FTA, or other safety analysis
  results as required by ISO 26262-9 clause 8. Documents hazard analysis and
  safety measures.
- ``implementations``: List of labels to Bazel targets representing the actual
  software implementation (cc_library, cc_binary, etc.) that realizes the
  component requirements. This is the source code that implements the safety
  functions as required by ISO 26262-6 clause 8.
- ``tests``: List of labels to Bazel test targets (cc_test, py_test, etc.)
  that verify the implementation against requirements. Includes unit tests and
  integration tests as required by ISO 26262-6 clause 9 for software unit
  verification.
- ``visibility``: Bazel visibility specification for the generated SEooC
  target. Controls which other packages can depend on this safety element.

**Generated Targets:**

This macro creates multiple targets automatically:

1. ``<name>_index``: Generates index.rst and conf.py files for the module
   documentation
2. ``<name>_seooc_index_lib``: Sphinx documentation library for the
   generated index
3. ``<name>``: The main SEooC target that aggregates all documentation
4. ``<name>.html``: Convenience target to build HTML documentation

**Implementation Details:**

The macro orchestrates several internal rules to:

- Generate a documentation index with a structured table of contents
- Organize documentation files under ``docs/safety_elements/<module_name>/``
- Integrate assumptions of use and component requirements into a unified documentation structure
- Provide Sphinx-compatible output for HTML generation

Private Rules
-------------

seooc
~~~~~

**File:** ``private/seooc.bzl``

**Purpose:** Internal rule that aggregates safety documentation artifacts
into a Sphinx-compatible structure.

**Implementation:** ``_seooc_build_impl``

**Functionality:**

- Collects documentation from ``assumptions_of_use`` and
  ``component_requirements`` dependencies
- Reorganizes file paths to place artifacts under
  ``docs/safety_elements/<module_name>/``
- Merges the generated index with artifact documentation
- Returns a ``SphinxDocsLibraryInfo`` provider for downstream consumption

**Attributes:**

- ``assumptions_of_use``: Label to assumptions of use documentation (mandatory)
- ``component_requirements``: Label to component requirements documentation (mandatory)
- ``index``: Label to the generated index file (mandatory)

**Output:**

Returns a ``SphinxDocsLibraryInfo`` provider containing:

- ``transitive``: Depset of documentation file structs with relocated paths
- ``files``: Empty list (files are in transitive dependencies)

seooc_sphinx_environment
~~~~~~~~~~~~~~~~~~~~~~~~

**File:** ``private/seooc_index.bzl``

**Purpose:** Generates the Sphinx environment files (index.rst and conf.py)
for a safety module.

**Implementation:** ``_seooc_sphinx_environment_impl``

**Functionality:**

- Creates a module-specific ``index.rst`` with:

  - Module name as header (uppercase with underline)
  - Table of contents (toctree) linking to all safety artifacts
  - References to assumptions of use and component requirements index files

- Generates a ``conf.py`` configuration file (currently placeholder content)

**Attributes:**

- ``module_name``: String name of the module (used for header generation)
- ``assumptions_of_use``: Label to assumptions documentation
- ``component_requirements``: Label to requirements documentation

**Generated Content Example:**

.. code-block:: rst

   MY_MODULE
   =========

   .. toctree::
      :maxdepth: 2
      :caption: Contents:

      assumptions_of_use/index
      component_requirements/index
      architectural_design/index
      safety_analysis/index

**Output:**

Returns ``DefaultInfo`` with generated ``index.rst`` files.

Documentation Structure
-----------------------

When using these rules, documentation is organized in the Bazel sandbox as
follows::

   docs/
   └── safety_elements/
       └── <module_name>/
           ├── index.rst (generated)
           ├── conf.py (generated)
           ├── assumptions_of_use/
           │   └── (user-provided documentation)
           ├── component_requirements/
           │   └── (user-provided documentation)
           ├── architectural_design/
           │   └── (user-provided documentation)
           └── safety_analysis/
               └── (user-provided documentation)

   bazel-bin/
   └── <module_name>/
       └── _sources/
           └── (generated documentation sources)

This structure reflects the file organization created in the Bazel sandbox
during the documentation generation process. The generated ``index.rst`` file
includes a table of contents that references all provided artifacts.

Integration with Sphinx
------------------------

The rules generate ``SphinxDocsLibraryInfo`` providers that are compatible
with ``@rules_python//sphinxdocs``. This enables:

- Automatic discovery of documentation files by Sphinx
- Proper path relocation for modular documentation
- Transitive dependency handling across multiple safety modules
- HTML, PDF, and other Sphinx output formats

Usage Example
-------------

Complete example in a BUILD file:

.. code-block:: python

   load("@baselibs//bazel/score_module:score_module.bzl",
        "safety_element_out_of_context")

   # Documentation artifacts
   sphinx_docs_library(
       name = "assumptions",
       srcs = ["docs/assumptions_of_use.rst"],
   )

   sphinx_docs_library(
       name = "requirements",
       srcs = ["docs/component_requirements.rst"],
   )

   sphinx_docs_library(
       name = "architecture",
       srcs = ["docs/architectural_design.rst"],
   )

   sphinx_docs_library(
       name = "safety",
       srcs = ["docs/safety_analysis.rst"],
   )

   # Implementation targets
   cc_library(
       name = "lifecycle_lib",
       srcs = ["lifecycle_manager.cpp"],
       hdrs = ["lifecycle_manager.h"],
   )

   # Test targets
   cc_test(
       name = "lifecycle_test",
       srcs = ["lifecycle_manager_test.cpp"],
       deps = [":lifecycle_lib"],
   )

   # Safety Element out of Context
   safety_element_out_of_context(
       name = "lifecycle_manager_seooc",
       assumptions_of_use = ":assumptions",
       component_requirements = ":requirements",
       architectural_design = ":architecture",
       safety_analysis = ":safety",
       implementations = [":lifecycle_lib"],
       tests = [":lifecycle_test"],
       visibility = ["//visibility:public"],
   )

Then build the documentation:

.. code-block:: bash

   # Build the SEooC target
   bazel build //:lifecycle_manager_seooc


Dependencies
------------

- ``@rules_python//sphinxdocs``: Sphinx documentation build rules
- ``SphinxDocsLibraryInfo``: Provider for documentation artifacts

Design Rationale
----------------

These rules enforce a structured approach to safety documentation by:

1. **Standardization**: All safety modules follow the same documentation
   structure
2. **Traceability**: Build system ensures all required artifacts are present
3. **Modularity**: Documentation can be composed from multiple sources
4. **Automation**: Index generation and path management are automated
5. **Integration**: Seamless integration with existing Sphinx workflows
