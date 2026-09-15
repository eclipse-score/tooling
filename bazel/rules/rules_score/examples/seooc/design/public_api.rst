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

Public API
==========

This is an **override** example (see ``architectural_design.rst``'s
"Authoring Pages Alongside Diagrams"): this file shares its stem with
``public_api.puml``, so it replaces that diagram's generated wrapper page
outright. The diagram is still staged as a sibling, so it can be embedded
here directly.

The SEooC exposes exactly one operation, ``GetNumber()``, on
``SampleLibraryAPI``.

.. uml:: public_api.puml
   :align: center
   :alt: SEooC example public API
