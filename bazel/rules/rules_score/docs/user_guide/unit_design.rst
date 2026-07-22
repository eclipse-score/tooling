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

The ``unit_design`` rule documents the **internal implementation** of a single
software unit — how its source code is structured, what data flows through it,
and how it behaves at the code level. This is distinct from the higher-level
architectural design diagrams (see :doc:`architectural_design`), which describe
the intended component structure of the SEooC as a whole.

A ``unit_design`` target is referenced by a ``unit`` target (see
:doc:`architectural_design` — *Implementation Architecture in Bazel*) to attach
code-level design artefacts to the unit.

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
