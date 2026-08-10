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

Optionally also emits a second "markers" .lobster file: one synthetic item
per forwarded entry, distinct from (but referencing) the original received
AoU item. This is what lets the dependable_element's own traceability report
show a "Forwarded AoUs" level with a `trace to: "Received AoUs"` edge —
using the original (identity-preserved) items directly would create a tag
collision with the "Received AoUs" level in the same report.

Reuses ``Requirement``, ``Tracing_Tag``, ``File_Reference``, ``lobster_read``,
and ``lobster_write`` from the lobster library (no manual JSON construction
or envelope/schema handling) — only the YAML parsing and the AoU-ID-to-tag
matching (which lobster itself has no concept of) are specific to this tool.
"""

from __future__ import annotations

import argparse
import logging
from pathlib import Path

import yaml
from lobster.common.errors import LOBSTER_Error, Message_Handler
from lobster.common.io import lobster_read, lobster_write
from lobster.common.items import Requirement, Tracing_Tag
from lobster.common.location import File_Reference

GENERATOR = "aou_forwarding_to_lobster"

logger = logging.getLogger(__name__)

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}


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
        raise SystemExit(f"YAML {yaml_path} must contain a 'forwarded_aous' key with a list of entries.")

    entries = data["forwarded_aous"]
    if not isinstance(entries, list):
        raise SystemExit(f"YAML {yaml_path}: 'forwarded_aous' must be a list.")

    result = []
    for i, entry in enumerate(entries):
        if not isinstance(entry, dict):
            raise SystemExit(f"YAML {yaml_path}: entry {i} must be a mapping with 'aou_id' and 'justification'.")
        aou_id = entry.get("aou_id")
        justification = entry.get("justification")
        if not aou_id:
            raise SystemExit(f"YAML {yaml_path}: entry {i} is missing required field 'aou_id'.")
        if not justification:
            raise SystemExit(
                f"YAML {yaml_path}: entry {i} (aou_id='{aou_id}') is missing required field 'justification'."
            )
        result.append({"aou_id": aou_id, "justification": justification})

    logger.info("Parsed %d forwarding entr%s from %s", len(result), "y" if len(result) == 1 else "ies", yaml_path)
    return result


def load_lobster_items(lobster_paths: list[str]) -> list[Requirement]:
    """Load all Requirement items from one or more .lobster JSON files.

    Args:
        lobster_paths: Paths to .lobster files.

    Returns:
        List of all Requirement items from all files.

    Raises:
        SystemExit: If a file cannot be read, or is not valid lobster-req-trace JSON.
    """
    mh = Message_Handler()
    all_items: list[Requirement] = []
    for path in lobster_paths:
        items: dict = {}
        try:
            lobster_read(mh, path, "aou", items)
        except (OSError, LOBSTER_Error) as e:
            raise SystemExit(f"Failed to parse lobster file {path}: {e}") from e
        all_items.extend(items.values())
    logger.info("Loaded %d lobster item(s) from %d file(s)", len(all_items), len(lobster_paths))
    return all_items


def _match_forwarded_entries(
    forwarding_entries: list[dict[str, str]],
    lobster_items: list[Requirement],
) -> list[tuple[dict[str, str], Requirement]]:
    """Match each forwarding YAML entry to its received AoU lobster item.

    Matches by checking if the AoU ID appears in the lobster item's tag.
    Lobster-trlc generates tags like "req PackageName.RecordName@version".
    The YAML can reference either the full versioned ID or the base name
    (without @version suffix).

    Args:
        forwarding_entries: Parsed YAML entries with 'aou_id' fields.
        lobster_items: All lobster items from received AoU files.

    Returns:
        List of (entry, matched item) pairs, in forwarding YAML order.

    Raises:
        SystemExit: If any aou_id from YAML doesn't match a received item.
    """
    # Build lookup: tag suffix -> item
    # Lobster-trlc may generate versioned tags like "req Pkg.Name@1".
    # We index by both the full ID and the base ID (without @version), but
    # only register the bare base ID when it is unambiguous: if two received
    # items share the same base ID at different versions (e.g. the same AoU
    # forwarded at different versions via different deps), a bare YAML
    # reference must not silently resolve to whichever one loaded last.
    item_by_id: dict[str, Requirement] = {}
    base_id_versions: dict[str, dict[object, Requirement]] = {}
    for item in lobster_items:
        full_id = item.tag.tag
        if item.tag.version:
            full_id += f"@{item.tag.version}"
        item_by_id[full_id] = item
        base_id_versions.setdefault(item.tag.tag, {})[item.tag.version] = item

    for base_id, by_version in base_id_versions.items():
        if len(by_version) == 1:
            item_by_id[base_id] = next(iter(by_version.values()))

    matched = []
    for entry in forwarding_entries:
        aou_id = entry["aou_id"]
        if aou_id not in item_by_id:
            available = ", ".join(sorted(item_by_id.keys())) if item_by_id else "(none)"
            hint = ""
            if aou_id in base_id_versions and len(base_id_versions[aou_id]) > 1:
                hint = " (ambiguous: multiple versions received -- specify '@version' explicitly)"
            raise SystemExit(
                f"AoU ID '{aou_id}' listed in forwarding YAML not found in received AoUs.{hint} Available IDs: {available}"
            )
        matched.append((entry, item_by_id[aou_id]))

    logger.info("Matched %d/%d forwarding entries to received AoU items", len(matched), len(forwarding_entries))
    return matched


def filter_forwarded_aous(
    forwarding_entries: list[dict[str, str]],
    lobster_items: list[Requirement],
) -> list[Requirement]:
    """Filter lobster items to only those listed in the forwarding YAML.

    The returned items are identical (same tag) to the originals: this
    output is handed on, unmodified, to this element's own dependees so
    that further chain-forwarding and eventual handling still resolves
    against the AoU's original tag.

    Args:
        forwarding_entries: Parsed YAML entries with 'aou_id' fields.
        lobster_items: All lobster items from received AoU files.

    Returns:
        Filtered list of lobster items matching the forwarding entries.

    Raises:
        SystemExit: If any aou_id from YAML doesn't match a received item.
    """
    return [item for _, item in _match_forwarded_entries(forwarding_entries, lobster_items)]


def build_forwarded_markers(
    forwarding_entries: list[dict[str, str]],
    lobster_items: list[Requirement],
    yaml_path: str,
) -> list[Requirement]:
    """Build synthetic "Forwarded AoUs" marker items for the DE's own report.

    Each marker is a distinct lobster item (its own tag, so it does not
    collide with the "Received AoUs" level in the same report) carrying a
    `refs` entry pointing at the original received AoU tag. This gives
    LOBSTER a `trace to: "Received AoUs"` edge for AoUs that are being
    chain-forwarded rather than handled locally. The forwarding
    justification becomes the marker's descriptive text.

    Args:
        forwarding_entries: Parsed YAML entries with 'aou_id' and
            'justification' fields.
        lobster_items: All lobster items from received AoU files.
        yaml_path: Path to the aou_forwarding.yaml file (used as the
            marker's source location).

    Returns:
        List of marker Requirement items, one per forwarding entry.

    Raises:
        SystemExit: If any aou_id from YAML doesn't match a received item.
    """
    markers = []
    for entry, item in _match_forwarded_entries(forwarding_entries, lobster_items):
        aou_id = entry["aou_id"]
        marker = Requirement(
            tag=Tracing_Tag("req", f"{aou_id}__forwarded"),
            location=File_Reference(yaml_path, line=1),
            framework="AoUForwarding",
            kind="ForwardedAoU",
            name=aou_id,
            text=entry["justification"],
        )
        marker.add_tracing_target(item.tag)
        markers.append(marker)
    return markers


def main() -> None:
    """Entry point for the AoU forwarding filter tool."""
    parser = argparse.ArgumentParser(description="Filter received AoU lobster entries for chain-forwarding.")
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
    parser.add_argument(
        "--markers-output",
        required=False,
        help="Optional output .lobster file path for synthetic 'Forwarded AoUs' "
        "marker items (distinct tags, refs pointing at the original received "
        "AoU items). Used by the dependable_element's own traceability report.",
    )
    parser.add_argument(
        "--log-level",
        choices=["error", "warn", "info", "debug"],
        default="warn",
        dest="log_level",
        help="Log level for tool output (default: warn).",
    )

    args = parser.parse_args()
    logging.basicConfig(level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s")

    # Parse YAML
    forwarding_entries = parse_forwarding_yaml(args.yaml)

    # Load received lobster items
    lobster_items = load_lobster_items(args.input_lobster)

    # Filter (identity-preserved copies, forwarded on to this element's own dependees)
    filtered_items = filter_forwarded_aous(forwarding_entries, lobster_items)

    # Write output
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        lobster_write(f, Requirement, GENERATOR, filtered_items)
    logger.info("Wrote %d item(s) to %s", len(filtered_items), output_path)

    if args.markers_output:
        markers = build_forwarded_markers(forwarding_entries, lobster_items, args.yaml)
        markers_output_path = Path(args.markers_output)
        markers_output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(markers_output_path, "w", encoding="utf-8") as f:
            lobster_write(f, Requirement, GENERATOR, markers)
        logger.info("Wrote %d marker item(s) to %s", len(markers), markers_output_path)


if __name__ == "__main__":
    main()
