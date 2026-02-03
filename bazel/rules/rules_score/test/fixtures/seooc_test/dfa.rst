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

Dependent Failure Analysis (DFA)
================================

This document contains the Dependent Failure Analysis (DFA) for the test SEooC module,
following ISO 26262 requirements for analysis of dependent failures.

Component DFA Overview
----------------------

The dependent failure analysis identifies and evaluates common cause failures,
cascading failures, and dependent failures that could affect the safety of the component.

.. comp_saf_dfa:: Common Cause Failure Analysis
   :id: comp_saf_dfa__seooc_test__common_cause_analysis
   :status: valid
   :tags: dfa, safety, seooc_test
   :violates: comp_arc_sta__seooc_test__data_processing_engine
   :failure_id: ccf_root
   :failure_effect: Common cause failures affecting multiple safety mechanisms simultaneously
   :mitigated_by: aou_req__seooc_test__controlled_environment
   :sufficient: yes

   **Analysis Scope**: Identification of common cause failures

   **Initiators Analyzed**:

   * Environmental conditions (temperature, EMI, vibration)
   * Power supply anomalies
   * Manufacturing and design defects
   * Maintenance-induced failures

   **Conclusion**: All identified common cause initiators have adequate mitigation measures.

.. comp_saf_dfa:: Power Supply Dependency
   :id: comp_saf_dfa__seooc_test__power_dependency
   :status: valid
   :tags: dfa, safety, seooc_test
   :violates: comp_arc_sta__seooc_test__data_processing_engine
   :failure_id: power_ccf
   :failure_effect: Power supply failure affecting both main and redundant processing paths
   :mitigated_by: aou_req__seooc_test__supply_voltage
   :sufficient: yes

   **Dependent Failure**: Power supply failure

   **Affected Elements**:

   * Main processing unit
   * Redundant calculation path
   * Communication interface

   **Independence Measures**:

   * Voltage monitoring with independent reference
   * Brownout detection circuit
   * Defined safe state on power loss

   **Residual Risk**: Acceptable (< 1e-8 per hour)

.. comp_saf_dfa:: Clock Source Dependency
   :id: comp_saf_dfa__seooc_test__clock_dependency
   :status: valid
   :tags: dfa, safety, seooc_test
   :violates: comp_arc_sta__seooc_test__data_processing_engine
   :failure_id: clock_ccf
   :failure_effect: Clock failure causing simultaneous malfunction of timing-dependent safety mechanisms
   :mitigated_by: comp_req__seooc_test__fault_detection
   :sufficient: yes

   **Dependent Failure**: Clock source failure

   **Affected Elements**:

   * Watchdog timer
   * Communication timing
   * Task scheduling

   **Independence Measures**:

   * Internal RC oscillator as backup
   * Clock monitoring unit
   * Frequency range checks

   **Residual Risk**: Acceptable (< 5e-9 per hour)

.. comp_saf_dfa:: Software Design Dependency
   :id: comp_saf_dfa__seooc_test__sw_design_dependency
   :status: valid
   :tags: dfa, safety, seooc_test
   :violates: comp_arc_sta__seooc_test__data_processing_engine
   :failure_id: sw_ccf
   :failure_effect: Systematic software defect in common code base affecting both calculation paths
   :mitigated_by: comp_req__seooc_test__redundant_calculation
   :sufficient: yes

   **Dependent Failure**: Systematic software defect

   **Affected Elements**:

   * Main calculation algorithm
   * Redundant calculation algorithm
   * Result comparison logic

   **Independence Measures**:

   * Diverse implementation of redundant path
   * Independent development teams
   * Different compilers/toolchains for each path

   **Residual Risk**: Acceptable (< 1e-7 per hour with diversity measures)

DFA Summary
-----------

.. comp_saf_dfa:: DFA Summary and Conclusion
   :id: comp_saf_dfa__seooc_test__dfa_summary
   :status: valid
   :tags: dfa, safety, seooc_test, summary
   :violates: comp_arc_sta__seooc_test__data_processing_engine
   :failure_id: dfa_summary
   :failure_effect: Combined dependent failure probability assessment
   :mitigated_by: aou_req__seooc_test__controlled_environment
   :sufficient: yes

   **Total Dependent Failure Probability**: < 1.5e-7 per hour

   **ASIL-B Target for Dependent Failures**: < 1e-6 per hour

   **Margin**: 6.7x

   **Status**: ✓ PASSED

   **Conclusion**: The component design provides adequate independence between
   safety mechanisms. All identified dependent failure modes have been analyzed
   and appropriate mitigation measures are in place.
