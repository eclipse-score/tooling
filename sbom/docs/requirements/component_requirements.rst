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

.. _sbom_component_requirements:

Component Requirements
######################

.. document:: SBOM Generator Component Requirements
   :id: doc__sbom_component_requirements
   :status: valid
   :safety: QM
   :security: NO
   :realizes: wp__requirements_comp


Metadata Provenance
===================

.. comp_req:: Component Checksum Automated Source
   :id: comp_req__sbom__checksum_automated_source
   :reqtype: Functional
   :security: NO
   :safety: QM
   :satisfies: feat_req__sbom__cisa_2025_minimum_elements
   :status: valid

   The generator shall source component SHA-256 checksums exclusively from
   the following automated inputs:

   - ``MODULE.bazel.lock`` ``registryFileHashes`` entries pointing to
     ``source.json`` files (for Bazel Central Registry modules), and
   - the ``sha256`` field of ``http_archive`` rules (for non-BCR
     dependencies).

   If neither source provides a checksum for a component, the hash field
   shall be omitted from that component's SBOM entry. Omitting the field is
   the correct output; emitting an incorrect or stale value is not permitted.


Output Format
=============

.. comp_req:: SPDX Output Version
   :id: comp_req__sbom__spdx_version
   :reqtype: Functional
   :security: NO
   :safety: QM
   :satisfies: feat_req__sbom__dual_format_output
   :status: valid

   The generator shall emit SPDX 2.3 compliant JSON. Migration to SPDX 3.0
   shall not be performed until SPDX 3.0 output is supported in production
   by at least one of the following downstream consumers: Trivy, GitHub
   Dependabot Dependency Submission API, or Grype.

   :rationale: SPDX 3.0 is a breaking JSON-LD rewrite of the format. As of
               February 2026 none of the major consumers support it, and the
               reference Python library (spdx-tools v0.8.4) describes its own
               3.0 support as experimental and not recommended for production.


.. comp_req:: CycloneDX Output Version
   :id: comp_req__sbom__cyclonedx_version
   :reqtype: Functional
   :security: NO
   :safety: QM
   :satisfies: feat_req__sbom__dual_format_output
   :status: valid

   The generator shall emit CycloneDX 1.6 compliant JSON with
   ``"$schema": "http://cyclonedx.org/schema/bom-1.6.schema.json"`` and
   ``"specVersion": "1.6"``.


.. needextend:: docname is not None and "sbom" in id
   :+tags: sbom
