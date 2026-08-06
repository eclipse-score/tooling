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

Software Unit Design
=====================

The ``unit_design`` documents the **internal implementation** of a single
software unit — how its source code is structured, what data flows through it,
and how it behaves at the code level. This is distinct from the higher-level
architectural design diagrams (see :doc:`architectural_design`), which describe
the intended component structure of the SEooC as a whole.

A ``unit_design`` is referenced by a ``unit`` target (see
:doc:`architectural_design` — *Implementation Architecture in Bazel*) to attach
code-level design artefacts to the unit.

The intent of a unit design is not to be a 1:1 representation of the code —
it is an abstraction that conveys the concept: the classes, relationships and
behaviour. It should mainly help to understand the unit´s intention and abstraction.

Designing the internals of a unit
----------------------------------

Before drawing the class diagram, decide *what belongs inside the unit*. Follow
the unit criteria in :doc:`architectural_design` (*Determining Components and
Units*). At the code level, aim for:

- **One cohesive responsibility per class** — the class diagram should read as a
  small set of classes that share data and collaborate for a single purpose. If
  two clusters of classes never reference each other, the unit is probably two
  units.
- **A narrow, intentional interface** — expose only the methods the unit's
  requirements and the internal/public API demand. Every public method becomes a
  test obligation and, for public interfaces, a safety-analysis entry point.
- **Explicit ownership of resources and lifetime** — model how objects are
  constructed and owned (e.g. ``unique_ptr`` composition, as ``Bar`` owns
  ``Foo`` below). Unclear ownership is a frequent source of failure modes.
- **Design is the contract** — the diagram is validated against the code:
  implementation-only members are allowed, but design-only members that do not
  exist in the code fail the build. Model what must be reviewed and traced; leave
  purely incidental helpers out.

Keep the unit design focused on structure that a reviewer needs to reason about
the unit's correctness and testability — not an exhaustive dump of every private
helper.

``unit_design`` — Code-Level Design Diagrams
-----------------------------------------------

The ``unit_design`` rule attaches PlantUML diagrams to a unit. It uses the same
``static`` / ``dynamic`` category split as ``architectural_design``, but scoped
to a single unit's implementation.

PlantUML
~~~~~~~~~

The example below is taken from ``examples/seooc``: ``unit_1`` implements a
class ``Foo``, and ``unit_2`` implements a class ``Bar`` that composes
``unit_1::Foo``.

.. uml:: ../_assets/SeoocExample_UnitClassDiagram.puml
   :align: center
   :alt: SEooC example unit class diagram

.. code-block:: text

    @startuml unit_class_diagram

    namespace unit_1 {
        class Foo <<final>>{
            --
            + GetNumber() : uint8_t
            + SetNumber(value : uint8_t) : void
        }
    }

    namespace unit_2 {
        class Bar <<final>>{
            --
            - foo_ : unique_ptr<unit_1::Foo>
            --
            + Bar(foo : unique_ptr<unit_1::Foo>)
            + AssertNumber() : bool
        }
    }

    Bar --> Foo : uses

    @enduml

Implementation
~~~~~~~~~~~~~~~

The class diagram above is generated from, and validated against, the real
unit implementation. ``foo.h`` declares the interface documented in the
diagram; ``// trace:`` comments tie each symbol to a requirement:

.. code-block:: cpp

    #ifndef FOO_H
    #define FOO_H

    #include <cstdint>

    namespace unit_1 {

    // trace: SampleComponent.REQ_COMP_002
    class Foo final {
    public:
      // trace: SampleComponent.REQ_COMP_001 SampleLibraryAPI.GetNumber
      std::uint8_t GetNumber() const;
      // trace: SampleLibraryAPI.SetNumber
      void SetNumber(std::uint8_t value);
    };

    } // namespace unit_1

    #endif // FOO_H

``foo.cpp`` provides the implementation:

.. code-block:: cpp

    #include "unit_1/foo.h"

    namespace unit_1 {

    // trace: SampleComponent.REQ_COMP_001 SampleLibraryAPI.GetNumber
    std::uint8_t Foo::GetNumber() const { return 42u; }
    } // namespace unit_1

Bazel
~~~~~~

.. code-block:: starlark

    load(
        "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
        "unit",
        "unit_design",
    )

    unit_design(
        name = "unit_design",
        static = glob(["*.puml", "*.rst"]),
    )

    cc_library(
        name = "unit_1_lib",
        srcs = ["foo.cpp"],
        hdrs = ["foo.h"],
    )

    unit(
        name           = "unit_1",
        scope          = ["//unit_1:unit_1_lib"],
        implementation = [":unit_1_lib"],
        unit_design    = ["//unit_1/docs:unit_design"],
        tests          = [":unit_1_test"],
    )

``unit_design`` Rule Reference
---------------------------------

For the complete ``unit_design`` attribute reference, see
:ref:`unit_design <rule-unit-design>` in the rule index.
