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

Step 0 — Setup
===============

Before adding any content, declare the ``dependable_element`` target itself. All
other steps in this tutorial fill in the fields left empty here.

BUILD
------

.. code-block:: starlark

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "dependable_element",
   )

   dependable_element(
       name = "my_element",
       integrity_level = "B",
       requirements = [],
       assumptions_of_use = [],
       architectural_design = [],
       components = [],
       dependability_analysis = [],
       tests = [],
       maturity = "development",
   )

→ Next: :doc:`requirements`
