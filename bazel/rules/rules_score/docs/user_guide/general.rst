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

General Information
===================

``rules_score`` provides a set of Bazel rules that help you build and document a
**Safety Element out of Context (SEooC)** — a safety-critical software component
developed independently and delivered with all the evidence needed for integration
into a safety-relevant system.

By declaring your workproducts (requirements, architecture, units, safety analysis)
as Bazel targets, ``rules_score`` automatically verifies traceability and consistency
of all workproducts and assembles them into a Sphinx HTML documentation including
the traceability report.

The Dependable Element Concept
--------------------------------

A *dependable element* is the top-level entity:

.. list-table::
   :header-rows: 1
   :widths: 35 65

   * - Artifact
     - What it contains
   * - Assumed System Requirements
     - System-level requirements given as constraints from the surrounding context
   * - Feature Requirements
     - Functional and safety requirements for this element
   * - Assumptions of Use
     - Conditions the integrating project must satisfy
   * - Forwarded AoUs
     - Assumptions of use received from dependencies that must be handled or forwarded further
   * - Architectural Design
     - Software Architectural Design in PlantUML
   * - Software Units and Components
     - Implementation targets linked to their design
   * - Dependability Analysis
     - FMEA, FTA diagrams and control measures

Architecture Overview
---------------------

The diagram below shows the full set of ``rules_score`` rules and how they relate
to each other.

.. uml:: ../_assets/rules_score_overview.puml
   :align: center
   :alt: Overview of rules_score architecture
   :width: 100%

Getting Started
---------------

New to ``rules_score``?  Work through the step-by-step
:doc:`tutorial/index` to build a minimal SEooC from scratch.

Rule Reference ``dependable_element``
--------------------------------------

For the complete ``dependable_element`` attribute reference, see
:ref:`dependable_element <rule-dependable-element>` in the rule index.

Automatic Validations
----------------------

``rules_score`` enforces the following constraints at **build time** — the build
fails if any of them are violated.
The validation logic is specified and tested via the
:doc:`../tool_reference/index`.

Architecture consistency
~~~~~~~~~~~~~~~~~~~~~~~~

The components and units declared in ``dependable_element.components`` are
compared against the static PlantUML diagrams in ``architectural_design``. Every
component or unit that appears in the implementation tree must also appear in the
architecture diagrams.

Certified scope
~~~~~~~~~~~~~~~

Every Bazel target that is transitively referenced through ``unit.implementation``
must fall within the package tree declared by the ``unit`` and ``component``
targets belonging to this element. External library dependencies that are not
safety-certified must not appear there.

When ``maturity = "development"`` is set, scope violations are printed as
warnings instead of failing the build. Switch back to ``"release"`` before
certification.

Integrity level
~~~~~~~~~~~~~~~

A ``dependable_element`` with ``integrity_level = "B"`` must not depend (via
``deps``) on another ``dependable_element`` with ``integrity_level = "A"``. The
hierarchy is D > C > B > A.
