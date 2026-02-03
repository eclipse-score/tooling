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

Static Architecture
===================

This file contains the static architectural design for the SEooC test component.

.. comp_arc_sta:: SEooC Test Component
   :id: comp_arc_sta__seooc_test__main
   :security: NO
   :safety: QM
   :status: valid
   :fulfils: comp_req__seooc_test__input_data_processing

   The main component of the SEooC test module providing data processing capabilities.

.. comp_arc_sta:: Data Processor
   :id: comp_arc_sta__seooc_test__data_processor
   :security: NO
   :safety: QM
   :status: valid
   :fulfils: comp_req__seooc_test__output_accuracy

   Sub-component responsible for processing input data and generating output.

.. comp_arc_sta:: Fault Handler
   :id: comp_arc_sta__seooc_test__fault_handler
   :security: NO
   :safety: ASIL_B
   :status: valid
   :fulfils: comp_req__seooc_test__fault_detection

   Sub-component responsible for detecting and handling fault conditions.
