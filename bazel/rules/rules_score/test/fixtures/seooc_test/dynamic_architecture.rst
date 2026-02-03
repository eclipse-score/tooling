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

Dynamic Architecture
====================

This file contains the dynamic architectural design for the SEooC test component.

.. comp_arc_dyn:: Data Processing Sequence
   :id: comp_arc_dyn__seooc_test__data_processing
   :security: NO
   :safety: QM
   :status: valid
   :fulfils: comp_req__seooc_test__input_data_processing

   Sequence diagram showing the data processing flow from input to output.

   .. uml::

      @startuml
      participant "Client" as client
      participant "SEooC Test Component" as main
      participant "Data Processor" as processor

      client -> main : processData(input)
      main -> processor : process(input)
      processor --> main : result
      main --> client : output
      @enduml

.. comp_arc_dyn:: Fault Handling Sequence
   :id: comp_arc_dyn__seooc_test__fault_handling
   :security: NO
   :safety: ASIL_B
   :status: valid
   :fulfils: comp_req__seooc_test__fault_detection

   Sequence diagram showing the fault detection and safe state transition.

   .. uml::

      @startuml
      participant "Main Component" as main
      participant "Fault Handler" as fault
      participant "Safe State Manager" as safe

      main -> fault : checkHealth()
      alt fault detected
         fault -> safe : transitionToSafeState()
         safe --> fault : safeStateConfirmed
         fault --> main : faultHandled
      else no fault
         fault --> main : healthOK
      end
      @enduml
