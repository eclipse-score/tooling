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
Module E Documentation
======================

This is the documentation for Module E.

.. document:: Documentation for Module E
   :id: doc__module_fixtures_module_e
   :status: valid
   :safety: ASIL_B
   :security: NO
   :realizes:


Overview
--------

Module E depends only on Module B (no direct edge to Module C or Module D).
Its need reference below to a need defined in Module D is reachable only
via Module B -> Module D (and Module B -> Module C -> Module D) -- a
regression fixture for two-hop needs resolution: needs_external_needs.json
must be built from the full transitive closure of needs modules, not just
direct deps, or this reference can never resolve.

Two-hop need reference to Module D :need:`doc__module_fixtures_module_d`.

Features
--------

.. needlist::
   :tags: module_e
