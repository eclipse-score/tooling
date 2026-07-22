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

Overview
========

``rules_score`` organises safety-critical software artefacts into four groups:

**Documentation Rules** — Sphinx builder and supporting helpers:

- :ref:`sphinx_module <rule-sphinx-module>` — Builds Sphinx HTML from RST/MD sources with dependency merging
- :ref:`filter_execpath <rule-filter-execpath>` — Resolves a build output path into a Sphinx ``-D`` flag at analysis time *(advanced)*

**Artifact Rules** — declare individual process work products:

- :ref:`assumed_system_requirements <rule-assumed-system-req>` — System-level requirements received from the wider context
- :ref:`feature_requirements <rule-feature-requirements>` — High-level feature specifications
- :ref:`component_requirements <rule-component-requirements>` — Component-level requirements
- :ref:`assumptions_of_use <rule-assumptions-of-use>` — Safety-relevant operating conditions imposed on the integrator
- :ref:`glossary <rule-glossary>` — Glossary pages included in the generated documentation
- :ref:`architectural_design <rule-architectural-design>` — Software architecture (static, dynamic, public API)
- :ref:`unit_design <rule-unit-design>` — Code-level design diagrams scoped to a single unit
- :ref:`fmea <rule-fmea>` — Failure Mode and Effects Analysis (failure modes, FTA, control measures)
- :ref:`dependability_analysis <rule-dependability-analysis>` — Complete safety analysis wrapping one or more FMEA targets

**Structural Rules** — wire artefacts into a verifiable SEooC:

- :ref:`unit <rule-unit>` — Smallest testable software element (design + implementation + tests)
- :ref:`component <rule-component>` — Collection of units providing specific functionality
- :ref:`dependable_element <rule-dependable-element>` — Complete SEooC with all artefacts assembled and validated

All rules support cross-module dependencies for sphinx-needs integration and HTML merging.

Quick Reference
---------------

.. list-table::
   :header-rows: 1
   :widths: 28 20 52

   * - Rule
     - Category
     - User Guide
   * - :ref:`sphinx_module <rule-sphinx-module>`
     - Documentation
     - :doc:`integration_guide`
   * - :ref:`filter_execpath <rule-filter-execpath>`
     - Documentation
     - :ref:`Rule reference <rule-filter-execpath>` *(advanced)*
   * - :ref:`assumed_system_requirements <rule-assumed-system-req>`
     - Artifact
     - :doc:`user_guide/requirements`
   * - :ref:`feature_requirements <rule-feature-requirements>`
     - Artifact
     - :doc:`user_guide/requirements`
   * - :ref:`component_requirements <rule-component-requirements>`
     - Artifact
     - :doc:`user_guide/requirements`
   * - :ref:`assumptions_of_use <rule-assumptions-of-use>`
     - Artifact
     - :doc:`user_guide/assumptions_of_use`
   * - :ref:`glossary <rule-glossary>`
     - Artifact
     - :ref:`Rule reference <rule-glossary>`
   * - :ref:`architectural_design <rule-architectural-design>`
     - Artifact
     - :doc:`user_guide/architectural_design`
   * - :ref:`unit_design <rule-unit-design>`
     - Artifact
     - :doc:`user_guide/unit_design`
   * - :ref:`fmea <rule-fmea>`
     - Artifact
     - :doc:`user_guide/dependability_analysis`
   * - :ref:`dependability_analysis <rule-dependability-analysis>`
     - Artifact
     - :doc:`user_guide/dependability_analysis`
   * - :ref:`unit <rule-unit>`
     - Structural
     - :doc:`user_guide/architectural_design`
   * - :ref:`component <rule-component>`
     - Structural
     - :doc:`user_guide/architectural_design`
   * - :ref:`dependable_element <rule-dependable-element>`
     - Structural
     - :doc:`user_guide/general`

.. seealso::

   :doc:`User Guide <user_guide/index>` — step-by-step guides for every rule

   :doc:`Rule Reference <rule_reference>` — complete attribute reference for all rules
