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
"""
FTA PlantUML lobster linker.

Parses PlantUML FTA diagrams and extracts ``$TopEvent`` and ``$BasicEvent``
procedure calls, producing a ``.lobster`` file in ``lobster-act-trace`` format.

Each extracted item uses the *alias* (second argument) as its tag and name
because the alias is the TRLC fully-qualified name of the corresponding
safety-analysis record (e.g. ``SampleLibrary.SampleFailureMode``).  A ``refs``
entry pointing at ``req <alias>`` links the FTA artifact back to the
matching TRLC requirement in the traceability chain.

Supported call patterns (single-line, double-quoted args)::

    $TopEvent("Human readable name", "Namespace.RecordName")
    $BasicEvent("Human readable name", "Namespace.RecordName", "GateAlias")
"""

import argparse
import json
import logging
from pathlib import Path

logger = logging.getLogger(__name__)

LOBSTER_GENERATOR = "safety_analysis_tools"
LOBSTER_SCHEMA = "lobster-act-trace"
LOBSTER_VERSION = 3

# ---------------------------------------------------------------------------
# PlantUML call parser
# TODO: Replace with Plantuml Parser
# ---------------------------------------------------------------------------

# Procedure names and the indices of (name_arg, alias_arg) within their arg list.
_FTA_PROCEDURES: tuple[tuple[str, str, int, int], ...] = (
    ("$TopEvent", "TopEvent", 0, 1),
    ("$BasicEvent", "BasicEvent", 0, 1),
)


def _parse_quoted_args(line: str, proc_name: str) -> list[str] | None:
    """Extract double-quoted string arguments from a PlantUML procedure call.

    Finds ``proc_name(...)`` in *line*, then collects every double-quoted token
    inside the parentheses.  Returns ``None`` if the procedure is not present.
    """
    marker = proc_name + "("
    call_start = line.find(marker)
    if call_start == -1:
        return None
    paren_open = call_start + len(marker) - 1
    paren_close = line.find(")", paren_open + 1)
    if paren_close == -1:
        return None
    inside = line[paren_open + 1 : paren_close]
    args: list[str] = []
    pos = 0
    while pos < len(inside):
        q_open = inside.find('"', pos)
        if q_open == -1:
            break
        q_close = inside.find('"', q_open + 1)
        if q_close == -1:
            break
        args.append(inside[q_open + 1 : q_close])
        pos = q_close + 1
    return args if args else None


def _is_valid_trlc_fqn(alias: str) -> bool:
    """Return True when *alias* looks like ``Package.RecordName``."""
    parts = alias.split(".")
    if len(parts) != 2:
        return False
    return all(
        part
        and part[0].isalpha()
        or part[0] == "_"
        and all(c.isalnum() or c == "_" for c in part)
        for part in parts
    )


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------


def extract_fta_items(puml_file: str) -> list[dict]:
    """Parse a PlantUML FTA file and return lobster trace items.

    Args:
        puml_file: Path to the ``.puml`` file to parse.

    Returns:
        List of lobster item dicts in ``lobster-act-trace`` format.
    """
    path = Path(puml_file)
    try:
        content = path.read_text(encoding="utf-8")
    except OSError:
        logger.exception("Cannot read '%s'", puml_file)
        raise

    items: list[dict] = []

    for line_number, line in enumerate(content.splitlines(), start=1):
        for proc_name, kind, name_idx, alias_idx in _FTA_PROCEDURES:
            call_args = _parse_quoted_args(line, proc_name)
            if call_args is None or len(call_args) <= max(name_idx, alias_idx):
                continue
            name = call_args[name_idx]
            alias = call_args[alias_idx]
            if not _is_valid_trlc_fqn(alias):
                logger.warning(
                    "%s:%d: alias %r does not look like a valid "
                    "TRLC fully-qualified name (expected 'Package.Record')",
                    puml_file,
                    line_number,
                    alias,
                )
            items.append(
                {
                    "tag": f"fta {alias}",
                    "location": {
                        "kind": "file",
                        "file": str(path),
                        "line": line_number,
                        "column": None,
                    },
                    "name": alias,
                    "messages": [],
                    "just_up": [],
                    "just_down": [],
                    "just_global": [],
                    "refs": [f"req {alias}"],
                    "framework": "PlantUML",
                    "kind": kind,
                }
            )
            logger.debug(
                "Found %s: alias=%r name=%r at line %d",
                kind,
                alias,
                name,
                line_number,
            )
            break  # one match per line

    if not items:
        logger.warning("No FTA events found in '%s'", puml_file)

    return items


def create_lobster_output(items: list[dict]) -> dict:
    """Wrap items in the standard lobster JSON envelope."""
    return {
        "data": items,
        "generator": LOBSTER_GENERATOR,
        "schema": LOBSTER_SCHEMA,
        "version": LOBSTER_VERSION,
    }


# ---------------------------------------------------------------------------
# PlantUML preprocessor
# ---------------------------------------------------------------------------


def preprocess_puml(
    input_path: str,
    metamodel_path: str,
    output_path: str,
) -> None:
    """Inline ``!include fta_metamodel.puml`` into a PlantUML file.

    Replaces the ``!include fta_metamodel.puml`` directive with the content
    of the metamodel file (stripping outer ``@startuml`` / ``@enduml``
    markers so the combined file remains a valid PlantUML diagram).

    This avoids the need to colocate the metamodel alongside the diagram at
    build time and eliminates fragile shell ``cp`` actions in Bazel rules.

    Args:
        input_path:    Path to the source ``.puml`` file.
        metamodel_path: Path to ``fta_metamodel.puml``.
        output_path:   Path for the preprocessed output ``.puml``.
    """
    metamodel = Path(metamodel_path).read_text(encoding="utf-8")
    # Strip @startuml / @enduml so they don't nest inside the host diagram.
    meta_lines = [
        line
        for line in metamodel.splitlines(keepends=True)
        if line.strip() not in ("@startuml", "@enduml")
    ]
    meta_content = "".join(meta_lines)

    source = Path(input_path).read_text(encoding="utf-8")
    processed = source.replace("!include fta_metamodel.puml", meta_content)

    Path(output_path).write_text(processed, encoding="utf-8")
    logger.info("Preprocessed %s → %s", input_path, output_path)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(levelname)s: %(message)s",
    )

    parser = argparse.ArgumentParser(
        description="Inline fta_metamodel.puml into FTA diagrams and extract lobster traceability.",
    )
    parser.add_argument(
        "--metamodel",
        required=True,
        help="Path to fta_metamodel.puml to inline.",
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        dest="output_dir",
        help="Directory for the preprocessed .puml output files.",
    )
    parser.add_argument(
        "--lobster",
        required=True,
        help="Output .lobster traceability file path.",
    )
    parser.add_argument(
        "inputs",
        nargs="+",
        help="PlantUML FTA .puml files to process.",
    )

    _run_preprocess(parser.parse_args())


def _run_preprocess(args: argparse.Namespace) -> None:
    """Preprocess each diagram (inline metamodel) and extract lobster items."""
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    all_items: list[dict] = []
    for puml_file in args.inputs:
        preprocessed_path = output_dir / Path(puml_file).name
        preprocess_puml(puml_file, args.metamodel, str(preprocessed_path))
        items = extract_fta_items(puml_file)
        logger.info("Extracted %d item(s) from '%s'", len(items), puml_file)
        all_items.extend(items)

    lobster_output = create_lobster_output(all_items)
    with open(args.lobster, "w", encoding="utf-8") as fh:
        json.dump(lobster_output, fh, indent=2)
        fh.write("\n")
    logger.info("Wrote %d lobster item(s) to %s", len(all_items), args.lobster)


if __name__ == "__main__":
    main()
