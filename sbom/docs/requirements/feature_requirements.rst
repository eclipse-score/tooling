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

.. _sbom_feature_requirements:

Feature Requirements
####################

.. document:: SBOM Generator Feature Requirements
   :id: doc__sbom_feature_requirements
   :status: valid
   :safety: QM
   :security: NO
   :realizes: wp__requirements_feat


CISA 2025 Minimum Elements
===========================

.. feat_req:: CISA 2025 Mandatory SBOM Elements
   :id: feat_req__sbom__cisa_2025_minimum_elements
   :reqtype: Functional
   :security: NO
   :safety: QM
   :status: valid

   The SBOM generator shall produce output that contains all minimum elements
   mandated by CISA 2025 for every component entry: component name, component
   version, component hash (SHA-256), software identifier (PURL), license
   expression, dependency relationships, SBOM author, timestamp, tool name,
   and generation context (lifecycle phase).


Metadata Provenance
===================

.. feat_req:: Automated Metadata Sources
   :id: feat_req__sbom__automated_metadata_sources
   :reqtype: Process
   :security: NO
   :safety: QM
   :status: valid

   All field values written into generated SBOM output shall be derived
   exclusively from automated sources. No manually-curated static data,
   hardcoded lookup tables, or hand-edited cache files shall be used to
   supply values for any SBOM field.

Component Scope
===============

.. feat_req:: Build Target Dependency Scope
   :id: feat_req__sbom__build_target_scope
   :reqtype: Functional
   :security: NO
   :safety: QM
   :status: valid

   The SBOM shall include only components that are part of the transitive
   dependency closure of the declared build targets. Build-time tools that
   are not part of the delivered software (compilers, build systems, test
   frameworks, and code generation utilities) shall be excluded from the
   SBOM output.


Output Formats
==============

.. feat_req:: Dual Format SBOM Output
   :id: feat_req__sbom__dual_format_output
   :reqtype: Interface
   :security: NO
   :safety: QM
   :status: valid

   The SBOM generator shall produce output simultaneously in both SPDX 2.3
   JSON format and CycloneDX 1.6 JSON format from a single invocation.


.. needextend:: docname is not None and "sbom" in id
   :+tags: sbom
