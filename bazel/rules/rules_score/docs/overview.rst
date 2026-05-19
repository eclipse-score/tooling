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

**Documentation Rule** — the generic Sphinx builder underlying all other rules:

- :ref:`sphinx_module <rule-sphinx-module>` — Builds Sphinx HTML from RST/MD sources with dependency merging

**Artifact Rules** — declare individual process work products:

- :ref:`assumed_system_requirements <rule-assumed-system-req>` — System-level requirements received from the wider context
- :ref:`feature_requirements <rule-feature-requirements>` — High-level feature specifications
- :ref:`component_requirements <rule-component-requirements>` — Component-level requirements
- :ref:`assumptions_of_use <rule-assumptions-of-use>` — Safety-relevant operating conditions imposed on the integrator
- :ref:`architectural_design <rule-architectural-design>` — Software architecture (static, dynamic, public API)
- :ref:`unit_design <rule-unit-design>` — Code-level design diagrams scoped to a single unit
- :ref:`fmea <rule-fmea>` — Failure Mode and Effects Analysis (failure modes, FTA, control measures)
- :ref:`dependability_analysis <rule-dependability-analysis>` — Complete safety analysis wrapping one or more FMEA targets

**Structural Rules** — wire artefacts into a verifiable SEooC:

- :ref:`unit <rule-unit>` — Smallest testable software element (design + implementation + tests)
- :ref:`component <rule-component>` — Collection of units providing specific functionality
- :ref:`dependable_element <rule-dependable-element>` — Complete SEooC with all artefacts assembled and validated

All rules support cross-module dependencies for sphinx-needs integration and HTML merging.

Architecture diagram
--------------------

.. uml:: _assets/rules_score_overview.puml
   :align: center
   :alt: Overview of rules_score architecture
   :width: 100%
