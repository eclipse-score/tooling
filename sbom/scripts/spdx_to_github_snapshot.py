#!/usr/bin/env python3
"""Convert SPDX 2.3 JSON to GitHub Dependency Submission API snapshot format.

This script converts an SPDX 2.3 SBOM JSON file into the snapshot format
expected by the GitHub Dependency Submission API, enabling Dependabot
vulnerability alerts on dependencies declared in the SBOM.

GitHub Dependency Submission API:
  https://docs.github.com/en/rest/dependency-graph/dependency-submission

Usage:
    python3 spdx_to_github_snapshot.py \\
        --input my_sbom.spdx.json \\
        --output snapshot.json \\
        --sha <git-commit-sha> \\
        --ref refs/heads/main \\
        --job-correlator my-workflow_sbom \\
        --job-id <github-run-id>
"""

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DETECTOR_NAME = "score-sbom-generator"
DETECTOR_VERSION = "0.1.0"
DETECTOR_URL = "https://github.com/eclipse-score/tooling/tree/main/sbom"


def _extract_purl(package: dict[str, Any]) -> str | None:
    """Extract PURL from SPDX package externalRefs."""
    for ref in package.get("externalRefs", []):
        if ref.get("referenceType") == "purl":
            return ref.get("referenceLocator", "")
    return None


def _package_key(package: dict[str, Any]) -> str:
    """Return a stable key for a package (name@version or SPDXID)."""
    name = package.get("name", "")
    version = package.get("versionInfo", "")
    if name and version:
        return f"{name}@{version}"
    return package.get("SPDXID", name or "unknown")


def convert_spdx_to_snapshot(
    spdx: dict[str, Any],
    sha: str,
    ref: str,
    job_correlator: str,
    job_id: str,
) -> dict[str, Any]:
    """Convert SPDX 2.3 document to GitHub Dependency Submission snapshot.

    Args:
        spdx: Parsed SPDX 2.3 JSON document
        sha: Git commit SHA (40 hex chars)
        ref: Git ref (e.g. refs/heads/main)
        job_correlator: Unique string identifying the workflow + SBOM target
        job_id: GitHub Actions run ID (or any unique job identifier)

    Returns:
        GitHub Dependency Submission snapshot dict
    """
    packages_by_id: dict[str, dict[str, Any]] = {}
    for pkg in spdx.get("packages", []):
        spdx_id = pkg.get("SPDXID", "")
        if spdx_id:
            packages_by_id[spdx_id] = pkg

    # Find the root document package (DESCRIBES relationship target)
    relationships = spdx.get("relationships", [])
    root_ids: set[str] = set()
    direct_ids: set[str] = set()

    for rel in relationships:
        rel_type = rel.get("relationshipType", "")
        element = rel.get("spdxElementId", "")
        related = rel.get("relatedSpdxElement", "")

        if rel_type == "DESCRIBES":
            root_ids.add(related)
        elif rel_type in ("DEPENDS_ON", "DYNAMIC_LINK", "STATIC_LINK", "CONTAINS"):
            if element in root_ids:
                direct_ids.add(related)

    # Build dependency map: which packages depend on which
    depends_on: dict[str, list[str]] = {}
    for rel in relationships:
        rel_type = rel.get("relationshipType", "")
        element = rel.get("spdxElementId", "")
        related = rel.get("relatedSpdxElement", "")
        if rel_type in ("DEPENDS_ON", "DYNAMIC_LINK", "STATIC_LINK", "CONTAINS"):
            depends_on.setdefault(element, []).append(related)

    # Manifest name from SBOM document name or file name
    doc_name = spdx.get("name", "sbom")
    manifest_name = doc_name.replace(" ", "_").replace("/", "_")

    # Build resolved packages dict (exclude root/document descriptor packages)
    resolved: dict[str, dict[str, Any]] = {}

    for spdx_id, pkg in packages_by_id.items():
        # Skip the SBOM document itself (SPDXRef-DOCUMENT) and root component
        if spdx_id in root_ids or spdx_id == "SPDXRef-DOCUMENT":
            continue

        purl = _extract_purl(pkg)
        if not purl:
            # Skip packages without a PURL — Dependabot can't use them
            continue

        key = _package_key(pkg)

        # Relationship: direct if root explicitly depends on it, else indirect
        relationship = "direct" if spdx_id in direct_ids else "indirect"

        # Dependencies of this package
        dep_purls = []
        for dep_id in depends_on.get(spdx_id, []):
            dep_pkg = packages_by_id.get(dep_id)
            if dep_pkg:
                dep_purl = _extract_purl(dep_pkg)
                if dep_purl:
                    dep_purls.append(dep_purl)

        resolved[key] = {
            "package_url": purl,
            "relationship": relationship,
            "dependencies": dep_purls,
        }

    scanned = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    snapshot: dict[str, Any] = {
        "version": 0,
        "sha": sha,
        "ref": ref,
        "job": {
            "correlator": job_correlator,
            "id": job_id,
        },
        "detector": {
            "name": DETECTOR_NAME,
            "version": DETECTOR_VERSION,
            "url": DETECTOR_URL,
        },
        "scanned": scanned,
        "manifests": {
            manifest_name: {
                "name": manifest_name,
                "resolved": resolved,
            }
        },
    }

    return snapshot


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Convert SPDX 2.3 JSON to GitHub Dependency Submission snapshot"
    )
    parser.add_argument("--input", required=True, help="Path to SPDX 2.3 JSON file")
    parser.add_argument("--output", required=True, help="Output snapshot JSON path")
    parser.add_argument("--sha", required=True, help="Git commit SHA (40 hex chars)")
    parser.add_argument("--ref", required=True, help="Git ref (e.g. refs/heads/main)")
    parser.add_argument(
        "--job-correlator",
        default="score-sbom_sbom",
        help="Unique workflow+target identifier for Dependency Submission API",
    )
    parser.add_argument(
        "--job-id", default="0", help="GitHub Actions run ID (or unique job ID)"
    )
    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.exists():
        print(f"Error: input file not found: {input_path}", file=sys.stderr)
        return 1

    with input_path.open() as f:
        try:
            spdx = json.load(f)
        except json.JSONDecodeError as e:
            print(f"Error: invalid JSON in {input_path}: {e}", file=sys.stderr)
            return 1

    spdx_version = spdx.get("spdxVersion", "")
    if not spdx_version.startswith("SPDX-"):
        print(
            f"Warning: unexpected spdxVersion '{spdx_version}', expected SPDX-2.x",
            file=sys.stderr,
        )

    snapshot = convert_spdx_to_snapshot(
        spdx=spdx,
        sha=args.sha,
        ref=args.ref,
        job_correlator=args.job_correlator,
        job_id=args.job_id,
    )

    output_path = Path(args.output)
    with output_path.open("w") as f:
        json.dump(snapshot, f, indent=2)

    total_packages = sum(len(m["resolved"]) for m in snapshot["manifests"].values())
    print(
        f"Converted {len(spdx.get('packages', []))} SPDX packages → "
        f"{total_packages} Dependency Submission packages"
    )
    print(f"Output: {output_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
