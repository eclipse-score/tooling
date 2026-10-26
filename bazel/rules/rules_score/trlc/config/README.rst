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

ASIL Non-Degradation
--------------------

With ``QM < B < D``, ``FeatReq`` and ``CompReq`` checks (``trlc --verify``) reject a
requirement whose ``safety`` is lower than that of any upstream requirement it
references in ``derived_from``. Fault-tree ``RootCause`` upstreams carry no ASIL and
impose no constraint.

Tests for the checks live in ``test/``. TRLC reports ``union_single_type`` and
``union_partial_field_access`` lint warnings for the tuple item types that carry them;
these are expected.
