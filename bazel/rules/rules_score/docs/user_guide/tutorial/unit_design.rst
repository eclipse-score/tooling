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

Step 3 — Unit Design
======================

A ``unit`` bazel target is defined via its implementation, corresponding tests and design.
So in a first step the class / sequence diagrams for a sw unit need to be defined:

class_design.puml
------------------

.. literalinclude:: ../../examples/minimal/docs/class_design.puml
   :language: text
   :lines: 14-

src/my_unit.h
--------------

The public interface of the unit can be tied to its source symbols via a ``// trace:`` tag.

.. literalinclude:: ../../examples/minimal/src/my_unit.h
   :language: cpp
   :lines: 14-

src/my_unit.cpp
----------------

.. literalinclude:: ../../examples/minimal/src/my_unit.cpp
   :language: cpp
   :lines: 14-

BUILD
------

Add a ``unit_design`` target, wire up the implementation, and reference both
from the ``unit``:

.. code-block:: starlark

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "unit",
       "unit_design",
   )

   unit_design(
       name = "MyUnit_design",
       static = ["docs/class_design.puml"],
   )

   cc_library(
       name = "my_unit_lib",
       srcs = ["src/my_unit.cpp"],
       hdrs = ["src/my_unit.h"],
   )

   unit(
       name           = "MyUnit",
       implementation = [":my_unit_lib"],
       scope          = ["//:my_unit_lib"],
       unit_design    = [":MyUnit_design"],
       tests          = [],
   )

The ``unit_design.static`` attribute accepts PlantUML files (class, state,
object diagrams); use ``dynamic`` for sequence and activity diagrams.
``scope`` declares which targets the unit "owns" — targets outside the scope
that appear in the transitive implementation closure fail the scope check at
build time.

→ Full guide: :doc:`../unit_design`
