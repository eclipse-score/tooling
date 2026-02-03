Architectural Design
====================

This document describes the architectural design of the test SEooC module.

Software Architecture Overview
-------------------------------

The system consists of the following software components:

.. comp_arc_sta:: Input Processing Module
   :id: comp_arc_sta__seooc_test__input_processing_module
   :status: valid
   :tags: architecture, component, seooc_test
   :safety: ASIL_B
   :security: NO
   :fulfils: comp_req__seooc_test__input_data_processing, comp_req__seooc_test__can_message_reception

   Responsible for receiving and validating input data from CAN interface.

   **Inputs**: Raw CAN messages

   **Outputs**: Validated data structures

   **Safety Mechanisms**: CRC validation, sequence counter check

.. comp_arc_sta:: Data Processing Engine
   :id: comp_arc_sta__seooc_test__data_processing_engine
   :status: valid
   :tags: architecture, component, seooc_test
   :safety: ASIL_B
   :security: NO
   :fulfils: comp_req__seooc_test__output_accuracy, comp_req__seooc_test__redundant_calculation

   Core processing component that performs calculations on validated data.

   **Inputs**: Validated data from Input Processing Module

   **Outputs**: Processed results

   **Safety Mechanisms**: Dual-channel redundant calculation

.. comp_arc_sta:: Output Handler
   :id: comp_arc_sta__seooc_test__output_handler
   :status: valid
   :tags: architecture, component, seooc_test
   :safety: QM
   :security: NO
   :fulfils: comp_req__seooc_test__can_message_transmission

   Formats and transmits output data via CAN interface.

   **Inputs**: Processed results from Data Processing Engine

   **Outputs**: CAN messages

   **Safety Mechanisms**: Message sequence numbering, alive counter

.. comp_arc_sta:: Fault Detection and Handling
   :id: comp_arc_sta__seooc_test__fault_detection_handling
   :status: valid
   :tags: architecture, component, safety, seooc_test
   :safety: ASIL_B
   :security: NO
   :fulfils: comp_req__seooc_test__fault_detection, comp_req__seooc_test__safe_state_transition

   Monitors system health and handles fault conditions.

   **Inputs**: Status from all components

   **Outputs**: System state, error flags

   **Safety Mechanisms**: Watchdog timer, plausibility checks

Component Interfaces
---------------------

Interface: CAN Communication
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. real_arc_int:: CAN RX Interface
   :id: real_arc_int__seooc_test__can_rx
   :status: valid
   :tags: interface, seooc_test
   :safety: ASIL_B
   :security: NO
   :fulfils: comp_req__seooc_test__can_message_reception
   :language: cpp

   * **Protocol**: CAN 2.0B
   * **Baud Rate**: 500 kbps
   * **Message ID Range**: 0x100-0x1FF
   * **DLC**: 8 bytes

.. real_arc_int:: CAN TX Interface
   :id: real_arc_int__seooc_test__can_tx
   :status: valid
   :tags: interface, seooc_test
   :safety: ASIL_B
   :security: NO
   :fulfils: comp_req__seooc_test__can_message_transmission
   :language: cpp

   * **Protocol**: CAN 2.0B
   * **Baud Rate**: 500 kbps
   * **Message ID Range**: 0x200-0x2FF
   * **DLC**: 8 bytes

Design Decisions
----------------

.. comp_arc_dyn:: Use of Hardware Watchdog
   :id: comp_arc_dyn__seooc_test__hw_watchdog
   :status: valid
   :tags: design-decision, safety, seooc_test
   :safety: ASIL_B
   :security: NO
   :fulfils: comp_req__seooc_test__fault_detection

   The architecture includes a hardware watchdog timer to ensure system
   reliability and meet safety requirements.

   **Rationale**: Hardware watchdog provides independent monitoring
   of software execution and can detect timing violations.

   **Alternatives Considered**: Software-only monitoring (rejected due
   to lower ASIL coverage)

.. comp_arc_dyn:: Redundant Processing Paths
   :id: comp_arc_dyn__seooc_test__redundancy
   :status: valid
   :tags: design-decision, safety, seooc_test
   :safety: ASIL_B
   :security: NO
   :fulfils: comp_req__seooc_test__redundant_calculation

   Critical calculations are performed using redundant processing paths
   to detect and prevent silent data corruption.

   **Rationale**: Meets ASIL-B requirements for detection of random
   hardware faults during calculation.

   **Implementation**: Main path + shadow path with result comparison

Memory Architecture
-------------------

.. comp_arc_sta:: RAM Allocation
   :id: comp_arc_sta__seooc_test__ram_allocation
   :status: valid
   :tags: resource, memory, seooc_test
   :safety: QM
   :security: NO
   :fulfils: aou_req__seooc_test__memory_requirements

   * **Total RAM**: 512 KB
   * **Stack**: 64 KB
   * **Heap**: 128 KB
   * **Static Data**: 256 KB
   * **Reserved**: 64 KB

.. comp_arc_sta:: Flash Allocation
   :id: comp_arc_sta__seooc_test__flash_allocation
   :status: valid
   :tags: resource, memory, seooc_test
   :safety: QM
   :security: NO
   :fulfils: aou_req__seooc_test__memory_requirements

   * **Total Flash**: 2 MB
   * **Application Code**: 1.5 MB
   * **Configuration Data**: 256 KB
   * **Boot Loader**: 128 KB
   * **Reserved**: 128 KB
