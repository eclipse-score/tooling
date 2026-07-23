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

Step 2 — SW Architectural Design
================================

Once the requirements are in place, add a static architecture diagram that names:

- the SEooC
- its components, and
- its units

During the build process every plantuml diagram will be parsed and checked for consistency
with the implemented architecture / design. To enable this certain rules and guidelines have
to be followed while implementing the architecture diagram.

The conventions to define those architectural elements are displayed in the following example.

static_design.puml
------------------

.. literalinclude:: ../../examples/minimal/docs/static_design.puml
   :language: text
   :lines: 14-

BUILD
------

.. code-block:: starlark

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "architectural_design",
   )

   architectural_design(
       name = "my_arch",
       static = ["docs/static_design.puml"],
   )

After designing the target architecture in plantuml the real architecture has to be implemented
in Bazel. Therefore following bazel rules are available:

.. code-block:: starlark

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "component",
       "dependable_element",
       "unit",
   )

   dependable_element(
       name = "my_element",
       ...
       architectural_design = [":my_arch"],
       components = [":MyComponent"],
       requirements = [":feature_requirements"],
       ...
   )

      unit(
       name = "MyUnit",
       implementation = [],
       unit_design = [],
       tests = [],
   )

   component(
       name = "MyComponent",
       components = [":MyUnit"],
       requirements = [],
       tests = [],
   )

Once the architecture is in place, the build parses the PlantUML diagram and validates it against the Bazel model. It verifies element names, structural hierarchy, and consistency between the declared diagram and the implemented architecture.

→ Full guide: :doc:`../architectural_design`
