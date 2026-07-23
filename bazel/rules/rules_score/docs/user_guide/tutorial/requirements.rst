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

Step 1 — Requirements
======================

A ``dependable_element`` starts with TRLC requirement files and their
corresponding Bazel targets. Dependent on it´s size and complexity, a dependable element can have two
or three levels of requirements. This means that a component requirement can be linked both to an
assumed system and feature requirement.

Assumed System Requirements
----------------------------

``requirements/asr.trlc`` captures the constraints imposed on this element by the
surrounding system context:

.. literalinclude:: ../../examples/minimal/requirements/asr.trlc
   :language: text

Feature Requirements
---------------------

``requirements/feature_requirements.trlc`` lists the functional and safety
requirements for this element, each referencing its parent assumed system
requirement via ``derived_from``:

.. literalinclude:: ../../examples/minimal/requirements/feature_requirements.trlc
   :language: text

BUILD
------

.. code-block:: starlark

   load(
       "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
       "assumed_system_requirements",
       "dependable_element",
       "feature_requirements",
   )

   assumed_system_requirements(
       name = "assumed_system_requirements",
       srcs = ["requirements/asr.trlc"],
   )

   feature_requirements(
       name = "feature_requirements",
       srcs = ["requirements/feature_requirements.trlc"],
       deps = [":assumed_system_requirements"],
   )

   dependable_element(
       name = "my_element",
       integrity_level = "B",
       requirements = [":feature_requirements"],
       assumptions_of_use = [],
       architectural_design = [],
       components = [],
       dependability_analysis = [],
       tests = [],
   )

To validate TRLC syntax and Traceability within the requirements, each of the requirements
bazel targets exposes via a marco already a test target, which can be run via e.g.:
``bazel test //:assumed_system_requirements_test``.

→ Full guide: :doc:`../requirements`
