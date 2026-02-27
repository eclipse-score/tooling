#!/usr/bin/env python3
"""Generate cpp_metadata.json cache from cdxgen CycloneDX output.

Usage:
    # Generate from cdxgen output:
    npx @cyclonedx/cdxgen -t cpp --deep -r -o cdxgen_output.cdx.json
    python3 generate_cpp_metadata_cache.py cdxgen_output.cdx.json ../cpp_metadata.json

    # Or pipe directly:
    npx @cyclonedx/cdxgen -t cpp --deep -r | python3 generate_cpp_metadata_cache.py - ../cpp_metadata.json
"""

import argparse
import json
import sys


def convert_cdxgen_to_cache(cdxgen_path: str) -> dict:
    """Convert CycloneDX JSON from cdxgen to internal cache format."""
    if cdxgen_path == "-":
        cdx_data = json.load(sys.stdin)
    else:
        with open(cdxgen_path, encoding="utf-8") as f:
            cdx_data = json.load(f)

    if cdx_data.get("bomFormat") != "CycloneDX":
        print("Error: Input is not a CycloneDX JSON file", file=sys.stderr)
        sys.exit(1)

    cache = {}
    for comp in cdx_data.get("components", []):
        name = comp.get("name", "")
        if not name:
            continue

        entry = {
            "version": comp.get("version", "unknown"),
        }

        # License
        licenses = comp.get("licenses", [])
        if licenses:
            first = licenses[0]
            lic_obj = first.get("license", {})
            lic_id = lic_obj.get("id", "") or lic_obj.get("name", "")
            if not lic_id:
                lic_id = first.get("expression", "")
            if lic_id:
                entry["license"] = lic_id

        # Description
        if comp.get("description"):
            entry["description"] = comp["description"]

        # Supplier
        supplier = comp.get("supplier", {})
        if supplier and supplier.get("name"):
            entry["supplier"] = supplier["name"]
        elif comp.get("publisher"):
            entry["supplier"] = comp["publisher"]

        # PURL
        if comp.get("purl"):
            entry["purl"] = comp["purl"]

        # URL from externalReferences
        for ref in comp.get("externalReferences", []):
            if ref.get("type") in ("website", "distribution", "vcs") and ref.get("url"):
                entry["url"] = ref["url"]
                break

        cache[name] = entry

    return cache


def main():
    parser = argparse.ArgumentParser(
        description="Convert cdxgen CycloneDX output to cpp_metadata.json cache"
    )
    parser.add_argument("input", help="cdxgen CycloneDX JSON file (or - for stdin)")
    parser.add_argument(
        "output",
        nargs="?",
        default="cpp_metadata.json",
        help="Output cache file (default: cpp_metadata.json)",
    )
    parser.add_argument(
        "--merge",
        help="Merge with existing cache file (existing entries take precedence)",
    )
    args = parser.parse_args()

    cache = convert_cdxgen_to_cache(args.input)

    if args.merge:
        try:
            with open(args.merge, encoding="utf-8") as f:
                existing = json.load(f)
            # Existing entries take precedence
            for name, data in cache.items():
                if name not in existing:
                    existing[name] = data
            cache = existing
        except (OSError, json.JSONDecodeError):
            pass

    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(cache, f, indent=2, sort_keys=True)
        f.write("\n")

    print(f"Generated {args.output} with {len(cache)} C++ dependencies")


if __name__ == "__main__":
    main()
