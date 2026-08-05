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
"""Filter received AoU TRLC records for chain-forwarding.

TRLC-level analog of ``aou_forwarding_to_lobster.py``: reads the same
``aou_forwarding.yaml`` format and one or more received AoU ``.trlc`` files,
then emits, for each *input* file, a corresponding *output* file containing
only the AoU records from that input that are listed in the YAML (all other
records are dropped; a file with no matches becomes an empty-but-valid
``package X`` file). This lets a downstream ``component_requirements`` target
type-reference (from its ``derived_from`` field) an AoU that was
chain-forwarded through one or more intermediate dependable elements, while
records this element chose not to forward remain genuinely unresolvable to
that target -- not merely excluded from a report.

Every .trlc file in this codebase declares exactly one `package` per file, and
there is no API in the trlc Python library exposing the end/span of a
record's source text (``trlc.ast.Location`` only carries
``file_name``/``line_no``/``col_no``), so records cannot be safely extracted
by slicing raw source text (an AoU's ``description`` field is a
``Markup_String`` that could contain literal braces). Records are instead
re-serialized from their resolved field values
(``Record_Object.to_python_dict()``) via a small AoU-specific writer -- safe
specifically because this tool only ever handles ``AoU`` records (a small,
fixed field set), not arbitrary TRLC types.

The output file count/paths must be Bazel-declared at analysis time, before
the YAML (and thus the actual forwarding selection) can be read -- so output
files are paired 1:1 with input files (positionally, via --input-trlc /
--output-trlc) rather than grouped dynamically by origin package.

An input file may itself already be a previous stage's empty-but-valid
``package X`` placeholder (see above) with zero AoU records -- e.g. one
origin a middle element chose not to further-forward. Its package name is
then read directly from its source text instead of off a Record_Object.
"""

from __future__ import annotations

import argparse
import logging
import re
from pathlib import Path

import yaml
from trlc.ast import Record_Object
from trlc.errors import Message_Handler
from trlc.trlc import Source_Manager

logger = logging.getLogger(__name__)

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

_PACKAGE_DECL_RE = re.compile(r"^\s*package\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE)


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


def load_aou_records(spec_paths: list[str], input_trlc_paths: list[str]) -> list[Record_Object]:
    """Parse spec + input TRLC files and return all AoU record objects.

    Args:
        spec_paths: RSL files defining the requirements model (must define AoU).
        input_trlc_paths: Received AoU .trlc files to parse.

    Returns:
        All parsed record objects of type AoU, across all input files.

    Raises:
        SystemExit: If parsing fails.
    """
    mh = Message_Handler()
    sm = Source_Manager(mh)
    for path in spec_paths:
        sm.register_file(path)
    for path in input_trlc_paths:
        sm.register_file(path)

    stab = sm.process()
    if stab is None:
        raise SystemExit(f"Failed to parse TRLC files for AoU forwarding (spec={spec_paths}, input={input_trlc_paths})")

    records = [rec for rec in stab.iter_record_objects() if rec.n_typ.name == "AoU"]
    logger.info("Loaded %d AoU record(s) from %d input file(s)", len(records), len(input_trlc_paths))
    return records


def _match_forwarded_entries(
    forwarding_entries: list[dict[str, str]],
    records: list[Record_Object],
) -> list[tuple[dict[str, str], Record_Object]]:
    """Match each forwarding YAML entry to its received AoU record.

    Mirrors ``aou_forwarding_to_lobster._match_forwarded_entries``: matches
    by full versioned id (``Package.Name@version``) first, then base id
    (``Package.Name``).

    Args:
        forwarding_entries: Parsed YAML entries with 'aou_id' fields.
        records: All AoU record objects from received AoU files.

    Returns:
        List of (entry, matched record) pairs, in forwarding YAML order.

    Raises:
        SystemExit: If any aou_id from YAML doesn't match a received record.
    """
    record_by_id: dict[str, Record_Object] = {}
    for record in records:
        fqn = record.fully_qualified_name()
        version = record.to_python_dict()["version"]
        record_by_id[f"{fqn}@{version}"] = record
        record_by_id[fqn] = record

    matched = []
    for entry in forwarding_entries:
        aou_id = entry["aou_id"]
        if aou_id not in record_by_id:
            available = ", ".join(sorted(record_by_id.keys())) if record_by_id else "(none)"
            raise SystemExit(
                f"AoU ID '{aou_id}' listed in forwarding YAML not found in received AoUs. Available IDs: {available}"
            )
        matched.append((entry, record_by_id[aou_id]))

    logger.info("Matched %d/%d forwarding entries to received AoU records", len(matched), len(forwarding_entries))
    return matched


def _read_package_name(path: str) -> str:
    """Read a .trlc file's package name directly from its source text.

    Fallback for a file with zero AoU records, whose package can't be read
    off a Record_Object -- e.g. an upstream chain-forwarding stage's
    empty-but-valid ``package X`` placeholder being forwarded through
    another level. The file has already been successfully parsed by
    ``load_aou_records`` at this point, so a ``package`` declaration is
    guaranteed to be present.
    """
    text = Path(path).read_text(encoding="utf-8")
    match = _PACKAGE_DECL_RE.search(text)
    if not match:
        raise SystemExit(f"Received AoU file {path} has no records and no 'package' declaration found in its text.")
    return match.group(1)


def _trlc_string_literal(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def _serialize_aou(record: Record_Object) -> str:
    """Serialize a single AoU record object back into valid TRLC source text.

    Hardcoded to AoU's fixed field set (description, version, note, safety,
    mitigates) rather than a general-purpose TRLC pretty-printer -- see the
    module docstring for why this is safe. The frozen `status` field is
    intentionally omitted (always Status.valid; not re-assignable on an
    instance).
    """
    fields = record.to_python_dict()
    lines = [f"ScoreReq.AoU {record.name} {{"]
    lines.append(f"    description = {_trlc_string_literal(fields['description'])}")
    lines.append(f"    version = {fields['version']}")
    if fields.get("note") is not None:
        lines.append(f"    note = {_trlc_string_literal(fields['note'])}")
    lines.append(f"    safety = ScoreReq.Asil.{fields['safety']}")
    if fields.get("mitigates") is not None:
        lines.append(f"    mitigates = {_trlc_string_literal(fields['mitigates'])}")
    lines.append("}")
    return "\n".join(lines)


def build_forwarded_files(
    forwarding_entries: list[dict[str, str]],
    records: list[Record_Object],
    input_trlc_paths: list[str],
) -> dict[str, str]:
    """Build the output file content for each input file, keyed by input path.

    Args:
        forwarding_entries: Parsed YAML entries with 'aou_id' fields.
        records: All AoU record objects from received AoU files.
        input_trlc_paths: The received AoU .trlc file paths, in the order
            they must be paired with --output-trlc.

    Returns:
        Dict mapping each input path to the full text of the corresponding
        output file (only matched records from that input, or an
        empty-but-valid `package X` file if none matched).

    Raises:
        SystemExit: If any aou_id from YAML doesn't match a received record,
            or an input file has no records and no 'package' declaration can
            be found in its text either (should not happen for a file that
            already parsed successfully in load_aou_records).
    """
    matched_by_origin_file: dict[str, list[Record_Object]] = {path: [] for path in input_trlc_paths}
    for _, record in _match_forwarded_entries(forwarding_entries, records):
        matched_by_origin_file[record.location.file_name].append(record)

    records_by_origin_file: dict[str, list[Record_Object]] = {}
    for record in records:
        records_by_origin_file.setdefault(record.location.file_name, []).append(record)

    output_by_input: dict[str, str] = {}
    for path in input_trlc_paths:
        origin_records = records_by_origin_file.get(path)
        package_name = origin_records[0].n_package.name if origin_records else _read_package_name(path)

        matched = matched_by_origin_file[path]
        body = "\n\n".join(_serialize_aou(record) for record in matched)
        if body:
            output_by_input[path] = f"package {package_name}\nimport ScoreReq\n\n{body}\n"
        else:
            output_by_input[path] = f"package {package_name}\n"

    return output_by_input


def main() -> None:
    """Entry point for the AoU TRLC forwarding filter tool."""
    parser = argparse.ArgumentParser(description="Filter received AoU TRLC records for chain-forwarding.")
    parser.add_argument(
        "--yaml",
        required=True,
        help="Path to the aou_forwarding.yaml file listing AoU IDs to further-forward.",
    )
    parser.add_argument(
        "--spec",
        nargs="+",
        required=True,
        help="RSL spec file(s) defining the requirements model (must define AoU).",
    )
    parser.add_argument(
        "--input-trlc",
        nargs="+",
        required=True,
        help="One or more .trlc files received from deps containing AoU records.",
    )
    parser.add_argument(
        "--output-trlc",
        nargs="+",
        required=True,
        help="Output .trlc file paths, positionally paired 1:1 with --input-trlc.",
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

    if len(args.input_trlc) != len(args.output_trlc):
        raise SystemExit(
            f"--input-trlc ({len(args.input_trlc)} paths) and --output-trlc "
            f"({len(args.output_trlc)} paths) must have the same length."
        )

    forwarding_entries = parse_forwarding_yaml(args.yaml)
    records = load_aou_records(args.spec, args.input_trlc)
    output_by_input = build_forwarded_files(forwarding_entries, records, args.input_trlc)

    for input_path, output_path_str in zip(args.input_trlc, args.output_trlc):
        output_path = Path(output_path_str)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        content = output_by_input[input_path]
        output_path.write_text(content, encoding="utf-8")
        logger.info("Wrote %s (%d bytes) from %s", output_path, len(content), input_path)


if __name__ == "__main__":
    main()
