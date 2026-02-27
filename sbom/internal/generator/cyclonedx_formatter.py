"""CycloneDX 1.6 JSON formatter for SBOM generation.

This module generates CycloneDX 1.6 compliant JSON output from the component
information collected by the Bazel aspect and module extension.

CycloneDX 1.6 Specification: https://cyclonedx.org/docs/1.6/json/
"""

import re
import uuid
from typing import Any


def _normalize_spdx_license(expr: str) -> str:
    """Normalize SPDX boolean operators to uppercase as required by the spec.

    dash-license-scan returns lowercase operators (e.g. 'Apache-2.0 or MIT').
    SPDX 2.3 Appendix IV and CycloneDX 1.6 both require uppercase OR/AND/WITH.
    Uses space-delimited substitution to avoid modifying license identifiers
    that contain 'or'/'and' as substrings (e.g. GPL-2.0-or-later).
    """
    expr = re.sub(r" or ", " OR ", expr, flags=re.IGNORECASE)
    expr = re.sub(r" and ", " AND ", expr, flags=re.IGNORECASE)
    expr = re.sub(r" with ", " WITH ", expr, flags=re.IGNORECASE)
    return expr


def generate_cyclonedx(
    components: list[dict[str, Any]],
    config: dict[str, Any],
    timestamp: str,
    external_dep_edges: list[str] | None = None,
) -> dict[str, Any]:
    """Generate CycloneDX 1.6 JSON document.

    Args:
        components: List of component dictionaries
        config: Configuration dictionary with producer info
        timestamp: ISO 8601 timestamp

    Returns:
        CycloneDX 1.6 compliant dictionary
    """
    component_name = config.get("component_name", "unknown")
    component_version = config.get("component_version", "")
    producer_name = config.get("producer_name", "Eclipse Foundation")
    producer_url = config.get("producer_url", "")

    # Generate serial number (URN UUID)
    serial_number = f"urn:uuid:{uuid.uuid4()}"

    cdx_doc: dict[str, Any] = {
        "$schema": "https://cyclonedx.org/schema/bom-1.6.schema.json",
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": serial_number,
        "version": 1,
        "metadata": {
            "timestamp": timestamp,
            "tools": {
                "components": [
                    {
                        "type": "application",
                        "name": "score-sbom-generator",
                        "description": "Eclipse SCORE SBOM Generator (Bazel-native)",
                        "publisher": producer_name,
                    }
                ]
            },
            "component": {
                "type": "application",
                "name": component_name,
                "version": component_version if component_version else "unversioned",
                "bom-ref": _generate_bom_ref(component_name, component_version),
                "purl": f"pkg:github/eclipse-score/{component_name}@{component_version}"
                if component_version
                else None,
                "supplier": {
                    "name": producer_name,
                    "url": [producer_url] if producer_url else [],
                },
            },
            "supplier": {
                "name": producer_name,
                "url": [producer_url] if producer_url else [],
            },
        },
        "components": [],
        "dependencies": [],
    }

    # Clean up None values from metadata.component
    if cdx_doc["metadata"]["component"].get("purl") is None:
        del cdx_doc["metadata"]["component"]["purl"]

    # Add authors if provided
    authors = config.get("sbom_authors", [])
    if authors:
        cdx_doc["metadata"]["authors"] = [_author_entry(a) for a in authors]

    # Add generation lifecycle if provided
    generation_context = config.get("generation_context", "")
    if generation_context:
        cdx_doc["metadata"]["lifecycles"] = [{"phase": generation_context}]

    # Add extra tool names if provided
    extra_tools = config.get("sbom_tools", [])
    if extra_tools:
        for tool_name in extra_tools:
            cdx_doc["metadata"]["tools"]["components"].append(
                {
                    "type": "application",
                    "name": tool_name,
                }
            )

    # Root component bom-ref for dependencies
    root_bom_ref = _generate_bom_ref(component_name, component_version)

    # Add components
    dependency_refs = []
    for comp in components:
        cdx_component = _create_cdx_component(comp)
        cdx_doc["components"].append(cdx_component)
        dependency_refs.append(cdx_component["bom-ref"])

    # Build dependency graph
    depends_map: dict[str, set[str]] = {}
    if external_dep_edges:
        for edge in external_dep_edges:
            if "::" not in edge:
                continue
            src, dst = edge.split("::", 1)
            if not src or not dst:
                continue
            src_ref = _generate_bom_ref(src, _component_version_lookup(components, src))
            dst_ref = _generate_bom_ref(dst, _component_version_lookup(components, dst))
            depends_map.setdefault(src_ref, set()).add(dst_ref)

    # Add root dependency (main component depends on all components)
    cdx_doc["dependencies"].append(
        {
            "ref": root_bom_ref,
            "dependsOn": dependency_refs,
        }
    )

    # Add each component's dependency entry
    for comp in components:
        name = comp.get("name", "")
        version = comp.get("version", "")
        bom_ref = _generate_bom_ref(name, version)
        cdx_doc["dependencies"].append(
            {
                "ref": bom_ref,
                "dependsOn": sorted(depends_map.get(bom_ref, set())),
            }
        )

    return cdx_doc


def _create_cdx_component(component: dict[str, Any]) -> dict[str, Any]:
    """Create a CycloneDX component from component data.

    Args:
        component: Component dictionary

    Returns:
        CycloneDX component dictionary
    """
    name = component.get("name", "unknown")
    version = component.get("version", "unknown")
    purl = component.get("purl", "")
    license_id = _normalize_spdx_license(component.get("license", ""))
    description = component.get("description", "")
    supplier = component.get("supplier", "")
    comp_type = component.get("type", "library")
    source = component.get("source", "")
    url = component.get("url", "")
    checksum = component.get("checksum", "")
    cpe = component.get("cpe", "")
    aliases = component.get("aliases", [])
    pedigree_ancestors = component.get("pedigree_ancestors", [])
    pedigree_descendants = component.get("pedigree_descendants", [])
    pedigree_variants = component.get("pedigree_variants", [])
    pedigree_notes = component.get("pedigree_notes", "")

    cdx_comp: dict[str, Any] = {
        "type": _map_type_to_cdx_type(comp_type),
        "name": name,
        "version": version,
        "bom-ref": _generate_bom_ref(name, version),
    }

    # Add description
    if description:
        cdx_comp["description"] = description

    # Add PURL
    if purl:
        cdx_comp["purl"] = purl

    # Add license
    if license_id:
        if " AND " in license_id or " OR " in license_id:
            # Compound SPDX expression must use "expression", not "license.id"
            cdx_comp["licenses"] = [{"expression": license_id}]
        else:
            cdx_comp["licenses"] = [{"license": {"id": license_id}}]

    # Add supplier
    if supplier:
        cdx_comp["supplier"] = {
            "name": supplier,
        }

    # Add hashes (SHA-256 from Cargo.lock)
    if checksum:
        cdx_comp["hashes"] = [
            {
                "alg": "SHA-256",
                "content": checksum,
            }
        ]
    if cpe:
        cdx_comp["cpe"] = cpe

    if aliases:
        cdx_comp["properties"] = [
            {"name": "cdx:alias", "value": alias} for alias in aliases
        ]

    pedigree = _build_pedigree(
        pedigree_ancestors,
        pedigree_descendants,
        pedigree_variants,
        pedigree_notes,
    )
    if pedigree:
        cdx_comp["pedigree"] = pedigree

    # Add external references
    external_refs = []

    # Add download/source URL
    if url:
        external_refs.append(
            {
                "type": "distribution",
                "url": url,
            }
        )
    elif source == "crates.io":
        external_refs.append(
            {
                "type": "distribution",
                "url": f"https://crates.io/crates/{name}/{version}",
            }
        )

    # Add VCS URL for git sources
    if source == "git" and url:
        external_refs.append(
            {
                "type": "vcs",
                "url": url,
            }
        )

    if external_refs:
        cdx_comp["externalReferences"] = external_refs

    return cdx_comp


def _map_type_to_cdx_type(comp_type: str) -> str:
    """Map component type to CycloneDX component type.

    Args:
        comp_type: Component type string

    Returns:
        CycloneDX component type string
    """
    type_mapping = {
        "application": "application",
        "library": "library",
        "framework": "framework",
        "file": "file",
        "container": "container",
        "firmware": "firmware",
        "device": "device",
        "data": "data",
        "operating-system": "operating-system",
        "device-driver": "device-driver",
        "machine-learning-model": "machine-learning-model",
        "platform": "platform",
    }
    return type_mapping.get(comp_type, "library")


def _generate_bom_ref(name: str, version: str) -> str:
    """Generate a unique bom-ref for a component.

    Args:
        name: Component name
        version: Component version

    Returns:
        Unique bom-ref string
    """
    # Create a deterministic but unique reference
    sanitized_name = _sanitize_name(name)
    sanitized_version = _sanitize_name(version) if version else "unknown"
    return f"{sanitized_name}@{sanitized_version}"


def _sanitize_name(value: str) -> str:
    """Sanitize a string for use in bom-ref.

    Args:
        value: String to sanitize

    Returns:
        Sanitized string
    """
    result = []
    for char in value:
        if char.isalnum() or char in (".", "-", "_"):
            result.append(char)
        elif char in (" ", "/"):
            result.append("-")
    return "".join(result) or "unknown"


def _author_entry(value: str) -> dict[str, Any]:
    """Create author entry from a string."""
    value = value.strip()
    if "<" in value and ">" in value:
        name, rest = value.split("<", 1)
        email = rest.split(">", 1)[0].strip()
        return {"name": name.strip(), "email": email}
    return {"name": value}


def _build_pedigree(
    ancestors: list[str],
    descendants: list[str],
    variants: list[str],
    notes: str,
) -> dict[str, Any] | None:
    pedigree: dict[str, Any] = {}
    if ancestors:
        pedigree["ancestors"] = [_pedigree_ref(a) for a in ancestors]
    if descendants:
        pedigree["descendants"] = [_pedigree_ref(d) for d in descendants]
    if variants:
        pedigree["variants"] = [_pedigree_ref(v) for v in variants]
    if notes:
        pedigree["notes"] = notes
    return pedigree or None


def _pedigree_ref(value: str) -> dict[str, Any]:
    value = value.strip()
    if value.startswith("pkg:"):
        return {"purl": value}
    return {"name": value}


def _component_version_lookup(components: list[dict[str, Any]], name: str) -> str:
    for comp in components:
        if comp.get("name") == name:
            return comp.get("version", "")
    return ""
