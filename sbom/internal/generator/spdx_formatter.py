"""SPDX 2.3 JSON formatter for SBOM generation.

This module generates SPDX 2.3 compliant JSON output from the component
information collected by the Bazel aspect and module extension.

SPDX 2.3 Specification: https://spdx.github.io/spdx-spec/v2.3/
"""

import re
import uuid
from typing import Any

from sbom.internal.generator.utils import _normalize_spdx_license


def generate_spdx(
    components: list[dict[str, Any]],
    config: dict[str, Any],
    timestamp: str,
) -> dict[str, Any]:
    """Generate SPDX 2.3 JSON document.

    Args:
        components: List of component dictionaries
        config: Configuration dictionary with producer info
        timestamp: ISO 8601 timestamp

    Returns:
        SPDX 2.3 compliant dictionary
    """

    namespace = config.get("namespace", "https://eclipse.dev/score")
    component_name = config.get("component_name", "unknown")
    component_version = config.get("component_version", "")
    producer_name = config.get("producer_name", "Eclipse Foundation")

    doc_uuid = uuid.uuid4()

    packages: list[dict[str, Any]] = []
    relationships: list[dict[str, Any]] = []

    # Root package
    root_spdx_id = "SPDXRef-RootPackage"
    root_package: dict[str, Any] = {
        "SPDXID": root_spdx_id,
        "name": component_name,
        "versionInfo": component_version if component_version else "unversioned",
        "downloadLocation": "https://github.com/eclipse-score",
        "supplier": f"Organization: {producer_name}",
        "primaryPackagePurpose": "APPLICATION",
        "filesAnalyzed": False,
        "licenseConcluded": "NOASSERTION",
        "licenseDeclared": "NOASSERTION",
        "copyrightText": "NOASSERTION",
    }
    packages.append(root_package)

    # DESCRIBES relationship
    relationships.append(
        {
            "spdxElementId": "SPDXRef-DOCUMENT",
            "relationshipType": "DESCRIBES",
            "relatedSpdxElement": root_spdx_id,
        }
    )

    # Add dependency packages
    for comp in components:
        pkg, spdx_id = _create_spdx_package(comp)
        packages.append(pkg)

        # Root depends on each component
        relationships.append(
            {
                "spdxElementId": root_spdx_id,
                "relationshipType": "DEPENDS_ON",
                "relatedSpdxElement": spdx_id,
            }
        )

    # Collect LicenseRef-* identifiers used in packages and declare them
    extracted = _collect_extracted_license_infos(packages)

    doc: dict[str, Any] = {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": f"SBOM for {component_name}",
        "documentNamespace": f"{namespace}/spdx/{_sanitize_id(component_name)}-{doc_uuid}",
        "creationInfo": {
            "created": timestamp,
            "creators": [
                f"Organization: {producer_name}",
                "Tool: score-sbom-generator",
            ],
        },
        "packages": packages,
        "relationships": relationships,
    }

    if extracted:
        doc["hasExtractedLicensingInfos"] = extracted

    return doc


def _create_spdx_package(
    component: dict[str, Any],
) -> tuple[dict[str, Any], str]:
    """Create an SPDX 2.3 Package for a component.

    Args:
        component: Component dictionary

    Returns:
        Tuple of (SPDX Package dictionary, spdx_id string)
    """
    name = component.get("name", "unknown")
    version = component.get("version", "unknown")
    purl = component.get("purl", "")
    license_id = _normalize_spdx_license(component.get("license", ""))
    description = component.get("description", "")
    supplier = component.get("supplier", "")
    comp_type = component.get("type", "library")
    checksum = component.get("checksum", "")

    spdx_id = f"SPDXRef-{_sanitize_id(name)}-{_sanitize_id(version)}"

    # Determine download location
    url = component.get("url", "")
    source = component.get("source", "")
    if url:
        download_location = url
    elif source == "crates.io":
        download_location = f"https://crates.io/crates/{name}/{version}"
    else:
        download_location = "NOASSERTION"

    package: dict[str, Any] = {
        "SPDXID": spdx_id,
        "name": name,
        "versionInfo": version,
        "downloadLocation": download_location,
        "primaryPackagePurpose": _map_type_to_purpose(comp_type),
        "filesAnalyzed": False,
        "licenseConcluded": license_id if license_id else "NOASSERTION",
        "licenseDeclared": license_id if license_id else "NOASSERTION",
        "copyrightText": "NOASSERTION",
    }

    if checksum:
        package["checksums"] = [{"algorithm": "SHA256", "checksumValue": checksum}]

    if description:
        package["description"] = description

    if supplier:
        package["supplier"] = f"Organization: {supplier}"

    # Add PURL as external reference
    if purl:
        package["externalRefs"] = [
            {
                "referenceCategory": "PACKAGE-MANAGER",
                "referenceType": "purl",
                "referenceLocator": purl,
            },
        ]

    return package, spdx_id


def _map_type_to_purpose(comp_type: str) -> str:
    """Map component type to SPDX 2.3 primary package purpose."""
    type_mapping = {
        "application": "APPLICATION",
        "library": "LIBRARY",
        "framework": "FRAMEWORK",
        "file": "FILE",
        "container": "CONTAINER",
        "firmware": "FIRMWARE",
        "device": "DEVICE",
        "data": "DATA",
    }
    return type_mapping.get(comp_type, "LIBRARY")


def _collect_extracted_license_infos(
    packages: list[dict[str, Any]],
) -> list[dict[str, str]]:
    """Collect LicenseRef-* identifiers from packages and build declarations.

    SPDX requires every LicenseRef-* used in license expressions to be
    declared in hasExtractedLicensingInfos.

    Args:
        packages: List of SPDX package dicts

    Returns:
        List of extractedLicensingInfo entries
    """
    license_refs: set[str] = set()
    pattern = re.compile(r"LicenseRef-[A-Za-z0-9\-.]+")

    for pkg in packages:
        for field in ("licenseConcluded", "licenseDeclared"):
            value = pkg.get(field, "")
            license_refs.update(pattern.findall(value))

    return [
        {
            "licenseId": ref,
            "extractedText": f"See package metadata for license details ({ref})",
        }
        for ref in sorted(license_refs)
    ]


def _sanitize_id(value: str) -> str:
    """Sanitize a string for use in SPDX IDs.

    SPDX 2.3 IDs must match [a-zA-Z0-9.-]+
    """
    result = []
    for char in value:
        if char.isalnum() or char in (".", "-"):
            result.append(char)
        elif char in ("_", " ", "/", "@"):
            result.append("-")
    return "".join(result) or "unknown"
