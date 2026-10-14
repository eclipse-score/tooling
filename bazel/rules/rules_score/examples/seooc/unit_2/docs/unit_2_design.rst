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

Unit 2 Class Design
^^^^^^^^^^^^^^^^^^^

``unit_2`` implements ``Bar``, which composes a ``unit_1::Foo`` and validates
its value (see ``bar.h`` / ``bar.cpp``). The design is split in two files of
the same ``static`` view: the overview shows ``Foo``, ``Bar`` and their
cross-unit dependency, the detail file elaborates ``Bar`` without repeating the
namespace.

.. uml:: unit_2_overview.puml
   :align: center
   :alt: Unit 2 Overview
   :width: 100%

.. uml:: unit_2_class_diagram.puml
   :align: center
   :alt: Unit 2 Class Diagram
   :width: 100%
