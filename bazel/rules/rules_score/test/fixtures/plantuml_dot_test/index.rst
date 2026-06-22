..
    # *******************************************************************************
    # Copyright (c) 2025 Contributors to the Eclipse Foundation
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

PlantUML dot Rendering Test
===========================

This document verifies that the hermetic ``dot_builtins`` binary is correctly
wired to PlantUML via ``-graphvizdot`` and that ``@startdot`` content inside a
``.. uml::`` directive is rendered as an SVG image.

Simple DAG via @startdot
------------------------

.. uml::

   @startdot
   digraph {
       A -> B;
       A -> C;
       B -> D;
       C -> D;
       label = "Simple DAG";
   }
   @enddot

The diagram above should appear as an SVG in the produced HTML output.
