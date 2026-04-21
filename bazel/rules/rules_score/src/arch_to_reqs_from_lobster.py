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

"""Generate a lobster architecture item from component requirements.

Extracts requirement tags from component requirement .lobster files and
produces an architecture.lobster file with a single item representing the
component. The item references all component requirements allocated to this
component through Bazel (via TRLC).

Usage:
    python arch_to_reqs_from_lobster.py \\
        --component-name my_component \
        --req-lobster comp_reqs_1.lobster comp_reqs_2.lobster \
        --output architecture.lobster
"""

import argparse
import json
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate architecture lobster item for a component"
    )
    parser.add_argument(
        "--component-name",
        required=True,
        help="Name of the component Bazel target (e.g. my_component)",
    )
    parser.add_argument(
        "--req-lobster",
        nargs="+",
        required=True,
        help="Component requirement .lobster files to extract requirement tags from",
    )
    parser.add_argument(
        "--build-file",
        required=True,
        help="Workspace-relative path to the BUILD file defining this component (e.g. examples/seooc/BUILD)",
    )
    parser.add_argument(
        "--output",
        required=True,
        help="Path for the generated architecture .lobster file",
    )
    return parser.parse_args()


def extract_requirement_tags(lobster_files: list[str]) -> list[str]:
    """Extract all requirement tags from .lobster files.

    Args:
        lobster_files: Paths to .lobster JSON files with lobster-req-trace schema.

    Returns:
        Sorted list of unique requirement tag strings
        (e.g. "req SampleComponent.REQ_COMP_001").
    """
    tags: set[str] = set()
    for path in lobster_files:
        try:
            with open(path, encoding="utf-8") as f:
                data = json.load(f)
        except (json.JSONDecodeError, OSError) as e:
            raise SystemExit(f"Failed to parse {path}: {e}") from e
        for item in data.get("data", []):
            tag = item.get("tag")
            if tag:
                tags.add(tag)
    return sorted(tags)


def build_architecture_lobster(
    component_name: str, build_file: str, requirement_tags: list[str]
) -> dict:
    """Build the architecture lobster-imp-trace structure.

    Creates a single lobster implementation item representing the component.
    The item references all component requirement tags, reflecting the
    Bazel-level structural allocation.

    Args:
        component_name: Workspace-qualified label of the component (e.g. //pkg:name).
        build_file: Workspace-relative path to the BUILD file (e.g. examples/seooc/BUILD).
        requirement_tags: Requirement tag strings to reference.

    Returns:
        Dict representing the complete lobster-imp-trace JSON structure.
    """
    return {
        "data": [
            {
                "tag": f"arch {component_name}",
                "location": {
                    "kind": "file",
                    "file": build_file,
                    "line": 1,
                    "column": None,
                },
                "name": component_name,
                "messages": [],
                "just_up": [],
                "just_down": [],
                "just_global": [],
                "refs": requirement_tags,
                "language": "Bazel",
                "kind": "Architecture",
            }
        ],
        "generator": "score_arch_to_lobster",
        "schema": "lobster-imp-trace",
        "version": 3,
    }


def main() -> None:
    args = parse_args()
    req_tags = extract_requirement_tags(args.req_lobster)
    if not req_tags:
        message = (
            f"error: no requirement tags found in {args.req_lobster}; "
            "cannot generate an Architecture item with no requirement links"
        )
        raise SystemExit(message)
    arch_lobster = build_architecture_lobster(
        args.component_name, args.build_file, req_tags
    )
    Path(args.output).write_text(
        json.dumps(arch_lobster, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
