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

- **Static** — the structural organisation: which components and units exist, how they nest, and how they depend on each other. Validated against the Bazel model at build time.
- **Dynamic** — behavioural sequences, state transitions, and activity flows. Documentation only, not validated against Bazel targets.
- **Public API** — the interfaces the SEooC exposes to its environment, linked to safety analysis via ``FailureMode.interface``.
- **Internal API** — interfaces exposed between components inside the SEooC that are not part of the public boundary.

Static Architecture
--------------------

The static view describes the **structural organisation** of your software: what components and units exist, how they relate to each other, and which dependencies they carry. It is the primary input for the architecture consistency check.

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

PlantUML
~~~~~~~~~

Write a PlantUML class or component diagram that names every ``component`` and ``unit`` from your Bazel BUILD file.

.. uml:: ../_assets/SeoocExample_StaticDesign.puml
   :align: center
   :alt: SEooC example static architecture

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

    package "SampleLibraryAPI" as SampleLibraryAPI

    component_example --> SampleLibraryAPI

    @enduml

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

    package "SampleLibraryAPI" as SampleLibraryAPI

    component_example --> SampleLibraryAPI

    @enduml

Named Ports (alternative)
^^^^^^^^^^^^^^^^^^^^^^^^^^

When an element needs an explicitly named, standalone binding point — for example to distinguish multiple provided interfaces without attaching them to a specific child unit — declare a ``portin`` / ``portout`` inside the ``<<SEooC>>`` or ``<<component>>`` element instead of binding directly on a child element:

.. code-block:: text

    @startuml MySeooc_StaticDesign

    package "MySeooc" as MySeooc <<SEooC>> {
        component "KvsComponent" as KvsComponent <<component>> {
            component "KeyValueStore" as KeyValueStore <<unit>>
        }

        portin  " " as p_storage   ' required interface port
        portout " " as p_api       ' provided interface port
    }

    interface "score::storage" as storage
    interface "kvsapi"         as kvsapi

    p_storage -( storage : requires
    p_api     )- kvsapi  : provides

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

    # Unit for KeyValueStore
    cc_library(name = "kvs_lib",       srcs = ["kvs.cpp"],      hdrs = ["kvs.h"])
    cc_test   (name = "kvs_unit_test", srcs = ["kvs_test.cpp"], deps = [":kvs_lib"])

    unit(
        name           = "KeyValueStore",
        unit_design    = [":kvs_unit_design"],
        implementation = [":kvs_lib"],
        tests          = [":kvs_unit_test"],
    )

    # Unit for StorageBackend
    cc_library(name = "storage_lib",       srcs = ["storage_backend.cpp"], hdrs = ["storage_backend.h"])
    cc_test   (name = "storage_unit_test", srcs = ["storage_test.cpp"],   deps = [":storage_lib"])

    unit(
        name           = "StorageBackend",
        unit_design    = [":storage_unit_design"],
        implementation = [":storage_lib"],
        tests          = [":storage_unit_test"],
    )

component
^^^^^^^^^^

.. code-block:: starlark

    load("@score_tooling//bazel/rules/rules_score:rules_score.bzl",
         "component", "component_requirements")

    component_requirements(
        name = "kvs_comp_req",
        srcs = ["component_requirements.trlc"],
        deps = [":feature_req"],
    )

    # The component maps to KvsComponent in the PlantUML diagram
    component(
        name         = "KvsComponent",
        requirements = [":kvs_comp_req"],
        components   = [":KeyValueStore", ":StorageBackend"],
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

.. code-block:: text

    @startuml SeoocExample_DynamicDesign

    participant "Unit 1" as unit_1 <<unit>>
    participant "Unit 2" as unit_2 <<unit>>

    unit_1 -> unit_2 : GetData()
    unit_2 --> unit_1 : return : Data*

    @enduml

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

The public API view describes the **interface your SEooC exposes to its environment**. These diagrams are linked to safety analysis: ``FailureMode`` records reference interface items by name (via the ``interface`` field), enabling traceability from each failure mode back to the architecture.

PlantUML
~~~~~~~~~

.. uml:: ../_assets/SeoocExample_PublicApi.puml
   :align: center
   :alt: SEooC example public API

.. code-block:: text

    @startuml SeoocExample_PublicApi

    package "SampleLibraryAPI" as SampleLibraryAPI {
        interface "GetNumber" as GetNumber
    }

    @enduml

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

.. code-block:: text

    @startuml

    namespace safety_software_seooc_example {
      namespace component_example {
        interface "InternalInterface" as InternalInterface <<interface>> {
          {abstract} GetData(BindingType binding): Data*
        }
      }
    }

    @enduml

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
