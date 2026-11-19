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

S-CORE Requirements Metamodel
==============================

TRLC metamodel (``.rsl``) defining the requirement type hierarchy used throughout
the repository.

Type Hierarchy
--------------

::

    Requirement (abstract)
    ├── description: String
    ├── version: Integer
    ├── note: optional String
    ├── status: Status {valid, invalid}  -- frozen to "valid"
    │
    └── RequirementSafety (abstract, extends Requirement)
        ├── safety: Asil {QM, B, D}
        │
        ├── AssumedSystemReq
        │   └── rationale: String
        │
        ├── FeatReq
        │   └── derived_from: AssumedSystemReqId[1..*]
        │
        └── CompReq
            ├── derived_from (optional): FeatReqId[1..*]
            └── mitigates (optional): String

Usage
-----

Reference this metamodel as ``spec`` in ``trlc_requirements`` rules:

.. code-block:: starlark

    load("@trlc//:trlc.bzl", "trlc_requirements")

    trlc_requirements(
        name = "my_requirements",
        srcs = ["requirements.trlc"],
        spec = ["//tools/trlc/config:score_requirements_model"],
    )

Traceability
------------

Typed tuple IDs connect requirements across levels:
``AssumedSystemReq`` → ``FeatReq`` → ``CompReq``, forming the traceability chain
enforced by LOBSTER at the dependable-element level.

- ``FeatReq.derived_from`` accepts ``AssumedSystemReqId`` tuples (mandatory)
- ``CompReq.derived_from`` accepts ``FeatReqId`` tuples (optional)

ASIL Coverage
-------------

A parent requirement (``AssumedSystemReq`` or ``FeatReq``) with downstream requirements
needs at least one direct child (``FeatReq`` or ``CompReq`` whose ``derived_from`` names the
parent) with the same ``safety``. Further children may carry a different ASIL.

The rule needs the reverse relation (a record does not see its referrers), so it cannot be a
``checks`` block. It is checked by the ``asil_coverage_check`` plugin (``src/trlc_checks``) of
``//bazel/rules/rules_score:trlc_check_runner`` in ``dependable_element``.

TRLC reports ``union_single_type`` and ``union_partial_field_access`` lint warnings for the
tuple item types that carry them; these are expected.
