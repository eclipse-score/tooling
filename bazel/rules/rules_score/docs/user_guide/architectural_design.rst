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

Architectural Design
=====================

Declared vs. Implemented Architecture
---------------------------------------

- **Declared architecture** — the PlantUML diagrams passed to ``architectural_design`` (``static``, ``dynamic``, ``public_api``, ``internal_api``). This is what your architecture is *supposed* to look like: the components, units, and interfaces you intend to build, and how they should relate to each other.

- **Implemented architecture** — the actual Bazel targets that get compiled and tested: ``unit(implementation = [...])`` wraps the real source files, ``component(components = [...])`` groups those units, and ``dependable_element(components = [...])`` assembles the complete SEooC. This is what your architecture *actually* is.

Because these two views are authored independently, they can drift apart. Therefore ``rules_score`` implements an automatic **architecture consistency** check that runs at ``bazel build``/``bazel test`` time: every component or unit that appears in ``dependable_element.components`` must also appear, under the same name, in the static PlantUML diagram declared via ``architectural_design.static`` — and vice versa. A mismatch fails the build. See :doc:`general` for the full list of automatic validations ``rules_score`` performs.

Overview and Hierarchy
------------------------

Software in ``rules_score`` is structured in three levels:

::

    dependable_element   (SEooC — complete Safety Element out of Context)
    └── component        (groups units; owns component-level integration tests and requirements)
        ├── unit         (smallest independently verifiable architectural element: implementation + unit tests)
        └── component    (components can be nested for deeper hierarchies)
            └── unit

Two rules apply:

- ``unit`` targets must always be wrapped in a ``component`` — they cannot be placed directly under ``dependable_element``.
- ``component`` targets can be nested: a component may contain other components as well as units, allowing arbitrary depth.

Below the different levels there are multiple views which present the architecture from different perspectives:

- **Static** — the structural organisation: which components and units exist, how they nest, and how they depend on each other. Validated against the Bazel model at build time.
- **Dynamic** — behavioural sequences, state transitions, and activity flows. Documentation only, not validated against Bazel targets.
- **Public API** — the interfaces the SEooC exposes to its environment, linked to safety analysis via ``FailureMode.interface``.
- **Internal API** — interfaces exposed between components inside the SEooC that are not part of the public boundary.


Determining Components and Units
--------------------------------

The Bazel rules and the consistency check only verify that your declared and
implemented structure *match* — they cannot tell you whether the structure is
*good*. Deciding what becomes a ``component`` and what becomes a ``unit`` is a
design activity.

Start from the requirements and the public interface, then decompose top-down:
the ``dependable_element`` is fixed by the SEooC boundary (its public API), and
you refine it into components and units until every leaf is small enough to be
implemented and tested by one owner.

What makes a unit
~~~~~~~~~~~~~~~~~~

A **unit** is the smallest architectural element that is *independently
verifiable*. Model something as a unit when it satisfies all of the following:

- **Single responsibility** — it does one thing; you can state its purpose in a
  single sentence without using "and".
- **Independently verifiable** — its behaviour can be fully covered by unit
  tests through a narrow interface, without standing up the rest of the SEooC.
- **Cohesive implementation** — its source files (the ``cc_library`` behind
  ``unit.implementation``) change together and share the same data.
- **One owner** — a single team/person is responsible for its design and tests.
- **Backed by a unit design** — its internal class structure is documented and
  validated against the code via :doc:`unit_design`.

If a unit's class diagram grows several unrelated clusters of classes, or its
unit tests split into groups that never share fixtures, it is really two units.

What makes a component
~~~~~~~~~~~~~~~~~~~~~~~~

A **component** *groups* units (and possibly sub-components) that collaborate to
deliver a coherent piece of feature behaviour. Introduce a component when:

- Several units together realise **one feature** or provide **one internal
  interface** to the rest of the SEooC.
- The grouping owns behaviour that only emerges from unit *interaction* —
  captured by **component-level integration tests** and **component
  requirements** (``CompReq``).
- It gives you a stable boundary you can allocate requirements to and reason
  about in the safety analysis.

Nest a component inside another component only when the inner grouping has its
own meaningful interface and requirements; do not nest purely to mirror source
folders.

Deciding the boundaries
~~~~~~~~~~~~~~~~~~~~~~~~~

Use these heuristics — most are the classic **high-cohesion / low-coupling**
rules applied to the ``rules_score`` element levels:

- **Cohesion first** — put things that change together and share data in the
  same element; split things that change for different reasons.
- **Minimise the interface** — prefer a decomposition that yields the *fewest,
  narrowest* interfaces between elements. A boundary that needs a wide,
  chatty interface is usually in the wrong place.
- **Follow the requirement allocation** — a ``CompReq`` is allocated to exactly
  one component. If a candidate requirement naturally splits across two groups,
  that is a component boundary; if it lands entirely inside one group, keep it
  together.
- **Match the failure-containment goal** — a component/unit boundary is also a
  boundary for the safety analysis. Draw boundaries so that a failure can be
  argued about, and a control measure placed, at a single element (see
  :doc:`dependability_analysis`).
- **Keep units testable in isolation** — if you cannot unit-test a candidate
  unit without a second unit present, either merge them or introduce an
  interface (internal API) so the dependency can be substituted.

Public vs. internal interfaces
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

- An interface is **public API** when it is part of the SEooC's contract with
  its environment — it is bound from the ``<<SEooC>>`` element and feeds
  ``FailureMode.interface`` traceability. Keep the public API as small as the
  requirements allow; every public method is a contract with the user and a safety-analysis
  entry point.
- An interface is **internal API** when it exists only *between* components/units
  inside the SEooC. Model it inside the owning element's namespace. Promote an
  interface from internal to public only when an external requirement forces it.

Common anti-patterns
~~~~~~~~~~~~~~~~~~~~~~

- **Folder-driven decomposition** — creating a component per source directory
  instead of per feature/interface. Structure follows responsibility, not layout.
- **God unit** — one unit that accumulates unrelated responsibilities because it
  was the first one created. Split as soon as a second responsibility appears.
- **Anaemic component** — a component that only forwards calls and owns no
  integration tests or requirements. Either give it a real boundary or flatten it.
- **Leaky public API** — exposing an interface publicly for convenience. It then
  drags in unnecessary failure modes and AoUs.

Static Architecture
--------------------

The static view describes the **structural organisation** of your software: what components and units exist, how they relate to each other, and which dependencies they carry. It is the primary input for the architecture consistency check.

PlantUML
~~~~~~~~~

Write a PlantUML class or component diagram that names every ``component`` and ``unit`` from your Bazel BUILD file.

.. uml:: ../_assets/SeoocExample_StaticDesign.puml
   :align: center
   :alt: SEooC example static architecture

.. literalinclude:: ../_assets/SeoocExample_StaticDesign.puml
   :language: text
   :lines: 14-

Valid PlantUML Definitions
^^^^^^^^^^^^^^^^^^^^^^^^^^^

The validator identifies elements by their **stereotype**, not by the PlantUML keyword used. Both ``package`` and ``component`` keywords are accepted at each level.

.. list-table::
   :header-rows: 1

   * - Stereotype
     - Valid PlantUML keywords
     - Meaning
     - Bazel rule
   * - ``<<SEooC>>``
     - ``package``, ``component``
     - Safety Element out of Context boundary
     - ``dependable_element``
   * - ``<<component>>``
     - ``component``, ``package``
     - Architectural component
     - ``component``
   * - ``<<unit>>``
     - ``component``, ``package``
     - Leaf implementation unit
     - ``unit``

Interface Bindings
^^^^^^^^^^^^^^^^^^^

Any component-type element (``<<SEooC>>``, ``<<component>>``, or ``<<unit>>``) can bind directly to an interface using the lollipop syntax — also a dedicated port can be drawn.

.. code-block:: text

    @startuml static_design

    package "Safety Software SEooC Example" as safety_software_seooc_example <<SEooC>> {
        component "ComponentExample" as component_example <<component>> {
            component "Unit 1" as unit_1 <<unit>>
            component "Unit 2" as unit_2 <<unit>>
            component "Sub Component Example" as sub_component_example <<component>>

            interface "InternalInterface" as InternalInterface
            unit_1 -l-( InternalInterface
            unit_2 )-r- InternalInterface
        }
    }

    interface "SampleLibraryAPI" as SampleLibraryAPI

    safety_software_seooc_example )-d- SampleLibraryAPI

    @enduml

Named Ports (alternative)
^^^^^^^^^^^^^^^^^^^^^^^^^^

When an element needs an explicitly named, standalone binding point — for example to distinguish multiple provided interfaces without attaching them to a specific child unit — declare a ``portin`` / ``portout`` inside the ``<<SEooC>>`` or ``<<component>>`` element instead of binding directly on a child element:

.. code-block:: text

    @startuml SeoocExample_StaticDesign

    package "Safety Software SEooC Example" as safety_software_seooc_example <<SEooC>> {
        component "ComponentExample" as component_example <<component>> {
            component "Unit 1" as unit_1 <<unit>>
        }

        portin  " " as p_required   ' required interface port
        portout " " as p_public     ' provided interface port
    }

    interface "RequiredInterface"  as RequiredInterface
    interface "SampleLibraryAPI"   as SampleLibraryAPI

    p_required -( RequiredInterface : requires
    p_public   )- SampleLibraryAPI : provides

    @enduml

Bazel
~~~~~~

architectural_design
^^^^^^^^^^^^^^^^^^^^^

.. code-block:: starlark

    load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "architectural_design")

    architectural_design(
        name   = "my_arch",
        static = ["static_design.puml"],  # the static diagram above
        dynamic = ["sequence_design.puml"],
    )

unit
^^^^^

.. code-block:: starlark

    load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "unit")

    # Unit 1
    cc_library(name = "unit_1_lib",  srcs = ["foo.cpp"],      hdrs = ["foo.h"])
    cc_test   (name = "unit_1_test", srcs = ["foo_test.cpp"], deps = [":unit_1_lib"])

    unit(
        name           = "unit_1",
        unit_design    = ["//unit_1/docs:unit_design"],
        implementation = [":unit_1_lib"],
        tests          = [":unit_1_test"],
    )

    # Unit 2
    cc_library(name = "unit_2_lib",  srcs = ["bar.cpp"],      hdrs = ["bar.h"])
    cc_test   (name = "unit_2_test", srcs = ["bar_test.cpp"], deps = [":unit_2_lib"])

    unit(
        name           = "unit_2",
        unit_design    = ["//unit_2/docs:unit_design"],
        implementation = [":unit_2_lib"],
        tests          = [":unit_2_test"],
    )

component
^^^^^^^^^^

.. code-block:: starlark

    load("@score_tooling//bazel/rules/rules_score:rules_score.bzl",
         "component", "component_requirements")

    component_requirements(
        name = "component_requirements",
        srcs = ["component_requirements.trlc"],
        deps = [":feature_requirements"],
    )

    # The component maps to ComponentExample in the PlantUML diagram
    component(
        name         = "component_example",
        requirements = [":component_requirements"],
        components   = [":unit_1", ":unit_2"],
        tests        = [],
    )

Dynamic Architecture
----------------------

The dynamic view describes **behavioural aspects** — sequences of interactions, state transitions, and activity flows. Dynamic diagrams document how your software behaves at runtime. They are not validated against the Bazel structure at build time.

PlantUML
~~~~~~~~~

.. uml:: ../_assets/SeoocExample_DynamicDesign.puml
   :align: center
   :alt: SEooC example dynamic sequence

.. literalinclude:: ../_assets/SeoocExample_DynamicDesign.puml
   :language: text
   :lines: 14-

Bazel
~~~~~~

.. code-block:: starlark

    architectural_design(
        name    = "my_arch",
        static  = ["static_design.puml"],
        dynamic = ["sequence.puml"],
    )

Public API
------------

The public API view describes the **interface your SEooC exposes to its environment**. They define a clear interface
for the user of the dependable element and state which functions of the dependable element are carrying the safety
related information.

As a proof for their safety relevance for each public method a FMEA should be carried out. This is documented by linking each method to a safety analysis:
``FailureMode`` records reference interface items by name (via the ``interface`` field), enabling traceability from each failure mode back to the architecture.

PlantUML
~~~~~~~~~

.. uml:: ../_assets/SeoocExample_PublicApi.puml
   :align: center
   :alt: SEooC example public API

.. literalinclude:: ../_assets/SeoocExample_PublicApi.puml
   :language: text
   :lines: 14-

Bazel
~~~~~~

.. code-block:: starlark

    architectural_design(
        name       = "my_arch",
        public_api = ["public_api.puml"],
    )

The ``public_api`` attribute also generates traceability items that can be referenced by ``fmea`` targets (see :doc:`dependability_analysis`) via the ``arch_design`` attribute.

Internal API
--------------

The internal API view documents interfaces exposed **between components inside the SEooC** that are not part of the public boundary — for example, a service one component provides to a sibling component. These diagrams are parsed like static/dynamic views, but their FlatBuffers output is tracked separately via ``ArchitecturalDesignInfo.internal_api`` for downstream validation. Unlike ``public_api``, they do not generate failure-mode traceability items.

PlantUML
~~~~~~~~~

Model the interface inside the namespace of the owning component so its fully-qualified name reflects the containment hierarchy:

.. uml:: ../_assets/SeoocExample_InternalApi.puml
   :align: center
   :alt: SEooC example internal API

.. literalinclude:: ../_assets/SeoocExample_InternalApi.puml
   :language: text
   :lines: 14-

Bazel
~~~~~~

.. code-block:: starlark

    architectural_design(
        name         = "my_arch",
        internal_api = ["internal_api.puml"],
    )

.. _rst-and-markdown-wrappers:

RST and Markdown Wrappers
----------------------------

When you want to combine a diagram with text, create an RST or Markdown file that embeds the diagram using the ``.. uml::`` directive (RST) or the MyST equivalent.

**RST wrapper example:**

.. code-block:: rst

    Static Architecture
    -------------------

    The following diagram shows the component structure of MySeooc.

    .. uml:: MySeooc_StaticDesign.puml

Include both the wrapper file *and* the referenced ``.puml`` file in the same Bazel list — the build needs both:

.. code-block:: starlark

    architectural_design(
        name   = "my_arch",
        static = [
            "static_design.rst",          # wrapper with prose
            "MySeooc_StaticDesign.puml",  # diagram referenced by the wrapper
        ],
    )

Rule Reference: ``architectural_design``
-------------------------------------------

For the complete ``architectural_design`` attribute reference, see :ref:`architectural_design <rule-architectural-design>` in the rule index.
