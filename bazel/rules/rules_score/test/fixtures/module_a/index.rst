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
Module A Documentation
======================

This is the documentation for Module A.

.. document:: Documentation for Module A
   :id: doc__module_fixtures_module_a
   :status: valid
   :safety: ASIL_B
   :security: NO
   :realizes: wp__component_arch

Overview
--------

Module A is a simple module that depends on Module C.

Features
--------

.. needlist::
   :tags: module_a

Cross-Module References
-----------------------

General reference to Module C :external+module_c_lib:doc:`index`.

Need reference to Module C :need:`doc__module_fixtures_module_c`.

Need reference to Module B :need:`doc__module_fixtures_module_b`.
