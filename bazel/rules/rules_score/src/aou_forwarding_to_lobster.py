# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
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
"""Filter received AoU lobster entries for chain-forwarding.

Reads a chain-forwarding YAML file and one or more received AoU .lobster
files, then outputs a new .lobster file containing only the entries listed
in the YAML. This enables dependable elements to further-forward AoUs they
cannot handle to their own dependees.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import yaml


def parse_forwarding_yaml(yaml_path: str) -> list[dict[str, str]]:
    """Parse the AoU forwarding YAML file.

    Args:
        yaml_path: Path to the YAML file.

    Returns:
        List of dicts with 'aou_id' and 'justification' keys.

    Raises:
        SystemExit: If YAML is malformed or missing required fields.
    """
    try:
        with open(yaml_path, encoding="utf-8") as f:
            data = yaml.safe_load(f)
    except (OSError, yaml.YAMLError) as e:
        raise SystemExit(f"Failed to parse YAML {yaml_path}: {e}") from e

    if not isinstance(data, dict) or "forwarded_aous" not in data:
        raise SystemExit(
            f"YAML {yaml_path} must contain a 'forwarded_aous' key with a list of entries."
        )

    entries = data["forwarded_aous"]
    if not isinstance(entries, list):
        raise SystemExit(f"YAML {yaml_path}: 'forwarded_aous' must be a list.")

    result = []
    for i, entry in enumerate(entries):
        if not isinstance(entry, dict):
            raise SystemExit(
                f"YAML {yaml_path}: entry {i} must be a mapping with 'aou_id' and 'justification'."
            )
        aou_id = entry.get("aou_id")
        justification = entry.get("justification")
        if not aou_id:
            raise SystemExit(
                f"YAML {yaml_path}: entry {i} is missing required field 'aou_id'."
            )
        if not justification:
            raise SystemExit(
                f"YAML {yaml_path}: entry {i} (aou_id='{aou_id}') is missing required field 'justification'."
            )
        result.append({"aou_id": aou_id, "justification": justification})

    return result


def load_lobster_items(lobster_paths: list[str]) -> list[dict]:
    """Load all items from one or more .lobster JSON files.

    Args:
        lobster_paths: Paths to .lobster files.

    Returns:
        List of all lobster item dicts from all files.
    """
    all_items = []
    for path in lobster_paths:
        try:
            with open(path, encoding="utf-8") as f:
                data = json.load(f)
        except (json.JSONDecodeError, OSError) as e:
            raise SystemExit(f"Failed to parse lobster file {path}: {e}") from e
        all_items.extend(data.get("data", []))
    return all_items


def filter_forwarded_aous(
    forwarding_entries: list[dict[str, str]],
    lobster_items: list[dict],
) -> list[dict]:
    """Filter lobster items to only those listed in the forwarding YAML.

    Matches by checking if the AoU ID appears in the lobster item's tag.
    Lobster-trlc generates tags like "req PackageName.RecordName@version".
    The YAML can reference either the full versioned ID or the base name
    (without @version suffix).

    Args:
        forwarding_entries: Parsed YAML entries with 'aou_id' fields.
        lobster_items: All lobster items from received AoU files.

    Returns:
        Filtered list of lobster items matching the forwarding entries.

    Raises:
        SystemExit: If any aou_id from YAML doesn't match a received item.
    """
    # Build lookup: tag suffix -> item
    # Lobster-trlc may generate versioned tags like "req Pkg.Name@1".
    # We index by both the full ID and the base ID (without @version).
    item_by_id: dict[str, dict] = {}
    for item in lobster_items:
        tag = item.get("tag", "")
        # Tags are formatted as "req PackageName.RecordName[@version]"
        parts = tag.split(" ", 1)
        if len(parts) == 2:
            full_id = parts[1]
            item_by_id[full_id] = item
            # Also index by base name (strip @version suffix)
            base_id = full_id.split("@")[0]
            if base_id != full_id:
                item_by_id[base_id] = item

    filtered = []
    for entry in forwarding_entries:
        aou_id = entry["aou_id"]
        if aou_id not in item_by_id:
            available = ", ".join(sorted(item_by_id.keys())) if item_by_id else "(none)"
            raise SystemExit(
                f"AoU ID '{aou_id}' listed in forwarding YAML not found in received AoUs. "
                f"Available IDs: {available}"
            )
        filtered.append(item_by_id[aou_id])

    return filtered


def create_lobster_output(items: list[dict]) -> dict:
    """Wrap items in the standard lobster JSON envelope."""
    return {
        "schema": "lobster-req-trace",
        "version": 3,
        "generator": "aou_forwarding_to_lobster",
        "data": items,
    }


def main() -> None:
    """Entry point for the AoU forwarding filter tool."""
    parser = argparse.ArgumentParser(
        description="Filter received AoU lobster entries for chain-forwarding."
    )
    parser.add_argument(
        "--yaml",
        required=True,
        help="Path to the aou_forwarding.yaml file listing AoU IDs to further-forward.",
    )
    parser.add_argument(
        "--input-lobster",
        nargs="+",
        required=True,
        help="One or more .lobster files received from deps containing AoU entries.",
    )
    parser.add_argument(
        "--output",
        required=True,
        help="Output .lobster file path for the filtered entries.",
    )

    args = parser.parse_args()

    # Parse YAML
    forwarding_entries = parse_forwarding_yaml(args.yaml)

    # Load received lobster items
    lobster_items = load_lobster_items(args.input_lobster)

    # Filter
    filtered_items = filter_forwarded_aous(forwarding_entries, lobster_items)

    # Write output
    output = create_lobster_output(filtered_items)
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
