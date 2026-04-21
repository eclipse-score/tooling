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
    ├── status: Status {valid, invalid}
    │
    └── RequirementSafety (abstract, extends Requirement)
        ├── safety: Asil {QM, B, D}
        │
        ├── AssumedSystemReq
        │   └── rationale: String
        │
        ├── FeatReq
        │   └── derived_from: list of ReqId
        │
        └── CompReq
            └── derived_from: list of ReqId

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

``ReqId`` tuples (e.g., ``FeatReq.derived_from``) connect requirements across levels:
``AssumedSystemReq`` → ``FeatReq`` → ``CompReq``, forming the traceability chain
enforced by LOBSTER at the dependable-element level.
