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
Module D Documentation
======================

This is the documentation for Module D.

.. document:: Documentation for Module D
   :id: doc__module_fixtures_module_d
   :status: valid
   :safety: ASIL_B
   :security: NO
   :realizes:


Overview
--------

Module D is a base module with no dependencies, shared as a diamond
dependency by Module B and Module C (both depend on it, neither directly
via Module A) — used to regression-test that its HTML is merged exactly
once into Module A's published site.
Local need link: :need:`doc__module_fixtures_module_d`

Features
--------

.. needlist::
   :tags: module_d

Content
-------

Module D provides foundational functionality shared by Module B and Module C.
