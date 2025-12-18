Assumptions of Use
==================

This document describes the assumptions of use for the test SEooC module.

.. aou_req:: Operating Temperature Range
   :id: aou_req__seooc_test__operating_temperature_range
   :status: valid
   :tags: environment, iso26262, seooc_test
   :safety: ASIL_B
   :security: NO

   The SEooC shall operate within temperature range -40°C to +85°C.

.. aou_req:: Supply Voltage
   :id: aou_req__seooc_test__supply_voltage
   :status: valid
   :tags: power, iso26262, seooc_test
   :safety: ASIL_B
   :security: NO

   The SEooC shall operate with supply voltage 12V ±10%.

   Maximum current consumption: 2.5A

.. aou_req:: Processing Load
   :id: aou_req__seooc_test__processing_load
   :status: valid
   :tags: performance, iso26262, seooc_test
   :safety: ASIL_B
   :security: NO

   The maximum processing load shall not exceed 80% to ensure
   timing requirements are met.

Environmental Assumptions
-------------------------

.. aou_req:: Controlled Environment
   :id: aou_req__seooc_test__controlled_environment
   :status: valid
   :tags: environment, seooc_test
   :safety: ASIL_B
   :security: NO

   The system operates in a controlled automotive environment
   compliant with ISO 16750 standards.

.. aou_req:: Maintenance
   :id: aou_req__seooc_test__maintenance
   :status: valid
   :tags: maintenance, seooc_test
   :safety: ASIL_B
   :security: NO

   Regular maintenance is performed according to the maintenance
   schedule defined in the integration manual.

Integration Constraints
-----------------------

.. aou_req:: CAN Bus Interface
   :id: aou_req__seooc_test__can_bus_interface
   :status: valid
   :tags: interface, communication, seooc_test
   :safety: ASIL_B
   :security: NO

   The host system shall provide a CAN 2.0B compliant interface
   for communication with the SEooC.

.. aou_req:: Memory Requirements
   :id: aou_req__seooc_test__memory_requirements
   :status: valid
   :tags: resource, seooc_test
   :safety: ASIL_B
   :security: NO

   The host system shall provide at least 512KB of RAM and
   2MB of flash memory for the SEooC.
