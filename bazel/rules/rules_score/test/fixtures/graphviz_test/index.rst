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

Graphviz Rendering Test
=======================

This document tests the ``sphinx.ext.graphviz`` directive to ensure hermetic graphviz
integration is working correctly.

Simple DAG
----------

.. graphviz::
   :align: center

   digraph {
       A -> B;
       A -> C;
       B -> D;
       C -> D;
       label = "Simple DAG";
   }

This graphviz diagram should render as SVG in the produced HTML output.
