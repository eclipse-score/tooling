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

Rule Reference
==============

Documentation Rules
-------------------

.. _rule-sphinx-module:

sphinx_module
~~~~~~~~~~~~~

Builds Sphinx-based HTML documentation from RST/MD source files. Supports
cross-module dependencies and automatic HTML merging.

.. code-block:: python

   sphinx_module(
       name  = "my_docs",
       srcs  = glob(["docs/**/*.rst", "docs/**/*.md"]),
       index = "docs/index.rst",
       deps  = ["@external_module//:docs"],
   )

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name; also the output directory prefix
   * - ``srcs``
     - label list
     - yes
     - RST, MD, image, and PlantUML source files
   * - ``index``
     - label
     - yes
     - Path to the root ``index.rst``
   * - ``deps``
     - label list
     - no
     - Other ``sphinx_module`` or ``dependable_element`` targets for cross-referencing and HTML merging (default ``[]``)
   * - ``sphinx``
     - label
     - no
     - Override the Sphinx binary (default: toolchain-provided binary)
   * - ``testonly``
     - bool
     - no
     - If ``True``, only testonly targets may depend on this (default ``False``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:**

- ``<name>`` — HTML output under ``bazel-bin/<name>/html/``; use in ``deps`` of other ``sphinx_module`` targets.
- ``<name>_needs`` — ``needs.json`` produced by the needs builder; consumed transitively by downstream modules for cross-module ``{requirement:downstream-ref}`` resolution. See :ref:`two-phase-sphinx-build` for the full data-flow description.


Artifact Rules
--------------

All artifact rules produce a ``SphinxSourcesInfo`` provider (documentation page)
and carry typed traceability information for downstream structural rules.

.. _rule-assumed-system-req:

assumed_system_requirements
~~~~~~~~~~~~~~~~~~~~~~~~~~~

System-level requirements that the SEooC receives from its operational context.
These are too broad to be assigned to a single component.

.. code-block:: python

   assumed_system_requirements(
       name = "sys_req",
       srcs = ["docs/assumed_system_requirements.trlc"],
   )

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name
   * - ``srcs``
     - label list
     - yes
     - ``.trlc`` files containing ``AssumedSystemReq`` records
   * - ``deps``
     - label list
     - no
     - Reserved for consistency; unused at root level (default ``[]``)
   * - ``image_srcs``
     - label list
     - no
     - Image/diagram files (``.svg``, ``.png``, or ``.puml``) referenced from ``description`` fields via ``.. image::``/``.. uml::`` — see :ref:`requirements-images` (default ``[]``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility (default ``["//visibility:public"]``)

**Generated targets:** ``<name>`` (documentation), ``<name>_test`` (TRLC syntax/type validation)

.. _rule-feature-requirements:

feature_requirements
~~~~~~~~~~~~~~~~~~~~

High-level requirements derived from ``AssumedSystemReq``. Operate at the
integration level; can only be satisfied by multiple components working together.

.. code-block:: python

   feature_requirements(
       name = "features",
       srcs = ["docs/features.trlc"],
       deps = [":sys_req"],
   )

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name
   * - ``srcs``
     - label list
     - yes
     - ``.trlc`` files containing ``FeatReq`` records
   * - ``deps``
     - label list
     - no
     - ``assumed_system_requirements`` targets for ``derived_from`` cross-reference resolution (default ``[]``)
   * - ``image_srcs``
     - label list
     - no
     - Image/diagram files (``.svg``, ``.png``, or ``.puml``) referenced from ``description`` fields via ``.. image::``/``.. uml::`` — see :ref:`requirements-images` (default ``[]``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility (default ``["//visibility:public"]``)

**Generated targets:** ``<name>`` (documentation), ``<name>_test`` (TRLC validation including cross-reference checks)

.. _rule-component-requirements:

component_requirements
~~~~~~~~~~~~~~~~~~~~~~

Requirements assigned to exactly one component; directly implementable and
testable within that component.

.. code-block:: python

   component_requirements(
       name = "comp_req",
       srcs = ["docs/requirements.trlc"],
       deps = [":features"],
   )

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name
   * - ``srcs``
     - label list
     - yes
     - ``.trlc`` files containing ``CompReq`` records
   * - ``deps``
     - label list
     - no
     - ``feature_requirements`` or ``assumed_system_requirements`` targets for ``derived_from`` resolution (default ``[]``)
   * - ``image_srcs``
     - label list
     - no
     - Image/diagram files (``.svg``, ``.png``, or ``.puml``) referenced from ``description`` fields via ``.. image::``/``.. uml::`` — see :ref:`requirements-images` (default ``[]``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility (default ``["//visibility:public"]``)

**Generated targets:** ``<name>`` (documentation), ``<name>_test`` (TRLC validation)

.. _rule-assumptions-of-use:

assumptions_of_use
~~~~~~~~~~~~~~~~~~

Conditions that the *integrating project* must fulfil when using this SEooC.

.. code-block:: python

   assumptions_of_use(
       name         = "aous",
       srcs         = ["docs/assumptions.trlc"],
       requirements = [":features"],
   )

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name
   * - ``srcs``
     - label list
     - yes
     - ``.trlc`` files containing ``AoU`` records
   * - ``requirements``
     - label list
     - no
     - ``feature_requirements`` or ``component_requirements`` targets that these AoUs trace to (default ``[]``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:** ``<name>`` (documentation), ``<name>_test`` (TRLC validation)

.. _rule-glossary:

glossary
~~~~~~~~

Collects glossary pages for Sphinx and forwards them to downstream
documentation assembly (for example through ``dependable_element``).

.. code-block:: python

  glossary(
     name = "project_glossary",
     srcs = ["docs/glossary.rst"],
  )

Example glossary source (``.rst``):

.. code-block:: rst

   Glossary
   ========

   .. glossary::

      integrity level
         ASIL rating (QM, A, B, C, D) indicating required safety rigor.

      component
         Software unit with defined interfaces, implementation, and tests.

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name
   * - ``srcs``
     - label list
     - yes
     - ``.rst`` files containing glossary definitions
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:** ``<name>`` (documentation; no standalone test)

.. _rule-arch-design:

.. _rule-architectural-design:

architectural_design
~~~~~~~~~~~~~~~~~~~~

Bundles static, dynamic, and public-API architecture views into a single target.
Provides ``ArchitecturalDesignInfo`` consumed by ``dependable_element`` and ``fmea``.

.. code-block:: python

   architectural_design(
       name       = "arch",
       static     = ["docs/static_design.puml"],
       dynamic    = ["docs/sequence.puml"],
       public_api = ["docs/public_api.puml"],
   )

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name
   * - ``static``
     - label list
     - no
     - Static-view files (``.puml``, ``.rst``, ``.md``, ``.svg``, ``.png``) (default ``[]``)
   * - ``dynamic``
     - label list
     - no
     - Dynamic-view files (default ``[]``)
   * - ``public_api``
     - label list
     - no
     - Public-API diagram files (``.puml``); also generates traceability items for safety analysis (default ``[]``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:** ``<name>`` (provides ``ArchitecturalDesignInfo``; no standalone test — consistency is validated as part of ``bazel test //pkg:my_element``)

.. _rule-unit-design:

unit_design
~~~~~~~~~~~

Attaches code-level design diagrams to a ``unit`` target. Accepts the same
file types as ``architectural_design`` but scoped to a single unit's internal
implementation.

.. code-block:: python

   unit_design(
       name    = "my_unit_design",
       static  = ["class_diagram.rst", "class_diagram.puml"],
       dynamic = ["sequence_diagram.puml"],
   )

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name
   * - ``static``
     - label list
     - no
     - Static-view files (class diagrams, data-structure diagrams). When an ``.rst`` fragment references a ``.puml`` via ``.. uml::``, list both files together (default ``[]``)
   * - ``dynamic``
     - label list
     - no
     - Dynamic-view files (sequence, state diagrams) (default ``[]``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:** ``<name>`` (no standalone test; diagrams are consumed by the parent ``unit``)

.. _rule-fmea:

fmea
~~~~

Bundles failure modes, control measures, and FTA diagrams into a single FMEA
documentation target.

.. code-block:: python

   fmea(
       name            = "my_fmea",
       failuremodes    = ["docs/failuremodes.trlc"],
       controlmeasures = ["docs/controlmeasures.trlc"],
       root_causes     = ["docs/fta.puml"],
       arch_design     = ":arch",
   )

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name
   * - ``failuremodes``
     - label list
     - no
     - ``.trlc`` files containing ``FailureMode`` records (default ``[]``)
   * - ``controlmeasures``
     - label list
     - no
     - ``.trlc`` files containing ``ControlMeasure`` records (default ``[]``)
   * - ``root_causes``
     - label list
     - no
     - FTA PlantUML diagram files (``.puml`` / ``.plantuml``) (default ``[]``)
   * - ``arch_design``
     - label
     - no
     - ``architectural_design`` target for interface traceability (default ``None``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:** ``<name>`` (documentation; no standalone test — traceability validated via the parent ``dependability_analysis``)

.. _rule-dependability-analysis:

dependability_analysis
~~~~~~~~~~~~~~~~~~~~~~

Wraps one or more ``fmea`` targets into a complete safety-analysis package.
Running ``bazel test`` validates the full FMEA traceability chain.

.. code-block:: python

   dependability_analysis(
       name = "analysis",
       fmea = [":my_fmea"],
   )

.. list-table::
   :header-rows: 1
   :widths: 18 12 10 60

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name
   * - ``fmea``
     - label list
     - yes
     - ``fmea`` targets to include in this analysis
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:** ``<name>`` (build → documentation; ``bazel test //pkg:analysis`` → full FMEA traceability validation)


Structural Rules
----------------

.. _rule-unit:

unit
~~~~

The smallest independently testable software element. Ties together
implementation targets, test targets, and an optional ``unit_design`` target.
The ``name`` must match the unit name used in the static architecture PlantUML
diagram.

.. code-block:: python

   unit(
       name           = "KeyValueStore",
       unit_design    = [":kvs_unit_design"],
       implementation = [":kvs_lib"],
       tests          = [":kvs_unit_test"],
   )

.. list-table::
   :header-rows: 1
   :widths: 22 12 10 56

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name; **must match** the ``<<unit>>`` name in the static PlantUML diagram
   * - ``unit_design``
     - label list
     - yes
     - ``unit_design`` targets describing the unit's internal design
   * - ``implementation``
     - label list
     - yes
     - Library or binary targets that implement this unit (e.g. ``cc_library``, ``py_library``)
   * - ``tests``
     - label list
     - yes
     - Test targets that verify this unit (may be ``[]``)
   * - ``scope``
     - label list
     - no
     - Additional targets needed by the implementation but not listed in ``implementation`` (default ``[]``)
   * - ``testonly``
     - bool
     - no
     - If ``True``, only testonly targets may depend on this unit (default ``True``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:** ``<name>`` (provides ``UnitInfo``; no standalone test — the listed ``tests`` run via ``bazel test //pkg:...`` directly)

.. _rule-component:

component
~~~~~~~~~

Groups ``unit`` (and optionally nested ``component``) targets into a logical
subsystem. Associates component-level requirements and integration tests.
The ``name`` must match the component name in the static architecture PlantUML
diagram.

.. code-block:: python

   component(
       name         = "KvsComponent",
       requirements = [":comp_req"],
       components   = [":KeyValueStore", ":StorageBackend"],
       tests        = [],
   )

.. list-table::
   :header-rows: 1
   :widths: 22 12 10 56

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name; **must match** the ``<<component>>`` name in the static PlantUML diagram
   * - ``requirements``
     - label list
     - yes
     - ``component_requirements`` or ``feature_requirements`` targets for this component
   * - ``components``
     - label list
     - no
     - Nested ``unit`` or ``component`` targets (default ``[]``)
   * - ``tests``
     - label list
     - yes
     - Integration test targets for the component as a whole (may be ``[]``)
   * - ``testonly``
     - bool
     - no
     - If ``True``, only testonly targets may depend on this component (default ``True``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:** ``<name>`` (provides ``ComponentInfo``; no standalone test — the listed ``tests`` run via ``bazel test //pkg:...`` directly)

.. _rule-dependable-element:

dependable_element
~~~~~~~~~~~~~~~~~~

Assembles all process artefacts into a complete SEooC. Runs Sphinx to generate
unified HTML documentation and enforces architecture consistency, traceability,
and scope checks at build/test time.

.. code-block:: python

   dependable_element(
       name                   = "my_seooc",
       integrity_level        = "B",
       assumptions_of_use     = [":aous"],
       requirements           = [":features"],
       architectural_design   = [":arch"],
       dependability_analysis = [":analysis"],
       components             = [":kvs_component"],
       tests                  = [],
   )

.. list-table::
   :header-rows: 1
   :widths: 22 12 10 56

   * - Attribute
     - Type
     - Required
     - Description
   * - ``name``
     - string
     - yes
     - Target name; also becomes the SEooC documentation title
   * - ``integrity_level``
     - string
     - yes
     - ``"A"``, ``"B"``, ``"C"``, or ``"D"`` (D is highest: D > C > B > A)
   * - ``assumptions_of_use``
     - label list
     - yes
     - ``assumptions_of_use`` targets
   * - ``requirements``
     - label list
     - yes
     - ``feature_requirements`` or ``assumed_system_requirements`` targets
   * - ``architectural_design``
     - label list
     - yes
     - ``architectural_design`` targets
   * - ``dependability_analysis``
     - label list
     - yes
     - ``dependability_analysis`` targets
   * - ``components``
     - label list
     - yes
     - ``component`` or ``unit`` targets that implement this SEooC
   * - ``tests``
     - label list
     - yes
     - System-level test targets (may be ``[]``)
   * - ``checklists``
     - label list
     - no
     - Additional ``.rst`` / ``.md`` checklist files (default ``[]``)
   * - ``deps``
     - label list
     - no
     - Other ``dependable_element`` targets for cross-referencing and HTML merging (default ``[]``)
   * - ``aou_forwarding``
     - label
     - no
     - A YAML file selecting which *received* AoUs to chain-forward to elements that depend on this one. Each entry requires an ``aou_id`` and a ``justification``. Own AoUs (from ``assumptions_of_use``) are always forwarded automatically.
   * - ``maturity``
     - string
     - no
     - ``"release"`` (default) or ``"development"`` — in development mode, scope violations and architecture errors are downgraded to warnings
   * - ``testonly``
     - bool
     - no
     - If ``True``, only testonly targets may depend on this element (default ``True``)
   * - ``visibility``
     - —
     - no
     - Bazel visibility

**Generated targets:**

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - Target
     - Purpose
   * - ``<name>``
     - Main target: build runs Sphinx; ``bazel test`` runs the traceability check
   * - ``<name>_doc``
     - Internal ``sphinx_module`` target; usable as ``deps`` in other Sphinx builds
   * - ``<name>_index``
     - Internal artefact-collection and architecture-validation target
