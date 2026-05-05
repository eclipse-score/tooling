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
"""RST requirement directive to TRLC converter."""

import argparse
import logging
import re
import sys
from pathlib import Path
from typing import Any

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

# Maps RST directive names to TRLC types in the S-CORE requirements model.
# Only directives that correspond to a concrete TRLC type in score_requirements_model.rsl
# are listed here.  All other directives in RST source files are silently skipped.
DIRECTIVE_TO_TRLC: dict[str, str] = {
    # Assumed System Requirements (root of the S-CORE traceability chain)
    "assumed_system_req": "ScoreReq.AssumedSystemReq",
    # Feature Requirements
    "feat_req": "ScoreReq.FeatReq",
    # Component Requirements
    "comp_req": "ScoreReq.CompReq",
    # Assumptions of Use
    "aou_req": "ScoreReq.AoU",
}

# Maps RST :safety: field values to ScoreReq.Asil enum literals.
# Values are transferred directly without any rounding or promotion logic.
SAFETY_MAP: dict[str, str] = {
    "QM": "ScoreReq.Asil.QM",
    "ASIL_A": "ScoreReq.Asil.A",
    "ASIL_B": "ScoreReq.Asil.B",
    "ASIL_C": "ScoreReq.Asil.C",
    "ASIL_D": "ScoreReq.Asil.D",
}

# RST fields that carry cross-package requirement references and are mapped to
# the TRLC ``derived_from`` attribute.  Only ``satisfies`` and ``derived_from``
# are used in the S-CORE process templates; all other relationship keywords
# (e.g. ``fulfils``, ``mitigates``) are not part of the TRLC model and are
# therefore excluded from this list.
_REF_FIELDS = ("satisfies", "derived_from")

# Explicit whitelist of RST field names that are transferred to TRLC.
# Every RST field not in this set is silently ignored during conversion.
# This keeps generated TRLC files independent of Sphinx-needs-only attributes
# (e.g. ``reqtype``, ``security``, ``valid_from``, ``belongs_to``, ``tags``).
_ALLOWED_RST_ATTRS: frozenset[str] = frozenset(
    {
        "id",  # → TRLC record name (not written as a field)
        "safety",  # → safety
        "satisfies",  # → derived_from cross-reference
        "derived_from",  # → derived_from cross-reference
        "rationale",  # → rationale (mandatory for AssumedSystemReq)
        "version",  # → version
    }
)

# TRLC types that require a rationale field.
_ASSUMED_SYSTEM_REQ_TYPES = {"ScoreReq.AssumedSystemReq"}

_DEFAULT_SAFETY = "QM"
_DEFAULT_VERSION = "1"
_DEFAULT_REF_PACKAGE = "TODO_PACKAGE"
_IMPORTS = ["ScoreReq"]

_RE_MARKUP = re.compile(r"\*\*?(.*?)\*\*?")
_RE_DIRECTIVE = re.compile(r"^\.\.\s+([\w]+)::\s*(.*)")
_RE_FIELD = re.compile(r"^\s+:([\w]+):\s*(.*)")  # noqa: E501

_TRLC_HEADER = """\
/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/"""


def _collect_fields(lines: list[str], i: int) -> tuple[dict[str, str], int]:
    """Read RST field list starting at line i. Return (fields dict, next line)."""
    fields: dict[str, str] = {}
    while i < len(lines):
        m = _RE_FIELD.match(lines[i])
        if m:
            fields[m.group(1)] = m.group(2).strip()
            i += 1
        elif not lines[i].strip():
            return fields, i + 1
        else:
            return fields, i
    return fields, i


def _collect_body(lines: list[str], i: int) -> tuple[str, int]:
    """Read indented body text starting at line i. Return (body text, next line)."""
    body_parts: list[str] = []
    while i < len(lines):
        line = lines[i]
        if line and not line[0].isspace():
            break
        if not line.strip():
            nxt = next(
                (lines[j] for j in range(i + 1, len(lines)) if lines[j].strip()), ""
            )
            if not nxt or not nxt[0].isspace():
                return " ".join(p for p in body_parts if p), i + 1
        body_parts.append(line.strip())
        i += 1
    return " ".join(p for p in body_parts if p), i


def _escape(text: str) -> str:
    """Escape a string for use inside TRLC double-quoted literals."""
    return text.replace("\\", "\\\\").replace('"', '\\"')


def _collect_refs(fields: dict[str, str]) -> list[str]:
    """Extract all cross-reference IDs from relationship fields."""
    return [
        r.strip()
        for k in _REF_FIELDS
        if k in fields
        for r in fields[k].split(",")
        if r.strip()
    ]


def parse_directives(content: str) -> list[dict[str, Any]]:
    """Parse supported requirement directives from RST content."""
    results: list[dict[str, Any]] = []
    lines = content.splitlines()
    i = 0
    while i < len(lines):
        m = _RE_DIRECTIVE.match(lines[i])
        if not m or m.group(1) not in DIRECTIVE_TO_TRLC:
            i += 1
            continue

        directive, title = m.group(1), m.group(2).strip()
        i += 1

        fields, i = _collect_fields(lines, i)
        raw_body, i = _collect_body(lines, i)
        body = _RE_MARKUP.sub(r"\1", raw_body).strip()

        results.append(
            {"directive": directive, "title": title, "fields": fields, "body": body}
        )
    return results


def render_trlc(
    directives: list[dict[str, Any]], package: str, ref_package: str
) -> str:
    """Render parsed directives into TRLC file content."""
    has_refs = any(_collect_refs(item["fields"]) for item in directives)
    imports = list(_IMPORTS)
    if (
        has_refs
        and ref_package
        and ref_package != _DEFAULT_REF_PACKAGE
        and ref_package not in imports
    ):
        imports.append(ref_package)
    import_lines = [f"import {name}" for name in imports]
    lines_out = [_TRLC_HEADER, f"package {package}", "", *import_lines, ""]

    for item in directives:
        fields = item["fields"]
        trlc_type = DIRECTIVE_TO_TRLC[item["directive"]]
        name = fields.get("id") or re.sub(r"\W+", "_", item["title"]).strip("_")
        safety = SAFETY_MAP.get(
            fields.get("safety", _DEFAULT_SAFETY).upper(),
            SAFETY_MAP[_DEFAULT_SAFETY],
        )
        desc = _escape(item["body"] or item["title"])

        lines_out.append(f"{trlc_type} {name} {{")
        lines_out.append(f'    description = "{desc}"')
        lines_out.append(f"    safety      = {safety}")

        refs = _collect_refs(fields)
        if refs:
            ref_list = ", ".join(f"{ref_package}.{r}@1" for r in refs)
            lines_out.append(f"    derived_from = [{ref_list}]")

        if trlc_type in _ASSUMED_SYSTEM_REQ_TYPES:
            rationale = fields.get("rationale", "TODO: add rationale")
            lines_out.append(f'    rationale   = "{_escape(rationale)}"')

        lines_out.append(f"    version     = {fields.get('version', _DEFAULT_VERSION)}")
        lines_out.append("}\n")

    return "\n".join(lines_out)


def convert(
    input_path: Path,
    output_path: Path,
    *,
    package: str | None = None,
    ref_package: str | None = None,
) -> int:
    """Convert one RST file to TRLC. Returns number of records written."""
    pkg = package or "".join(
        w.capitalize() for w in re.split(r"[_\-\s]+", input_path.stem)
    )
    directives = parse_directives(input_path.read_text(encoding="utf-8"))
    if not directives:
        logging.warning("no supported requirement directives found in %s", input_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        render_trlc(directives, pkg, ref_package or _DEFAULT_REF_PACKAGE),
        encoding="utf-8",
    )
    return len(directives)


if __name__ == "__main__":
    p = argparse.ArgumentParser(description="RST to TRLC converter")
    p.add_argument("input_file", type=Path)
    p.add_argument("--output-dir", type=Path, required=True)
    p.add_argument("--package", default=None)
    p.add_argument("--ref-package", default=None)
    p.add_argument(
        "--log-level",
        choices=["error", "warn", "info", "debug"],
        default="warn",
        dest="log_level",
        help="Log level for tool output (default: warn).",
    )
    args = p.parse_args()
    logging.basicConfig(
        level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s"
    )
    if not args.input_file.exists():
        sys.exit(f"ERROR: file not found: {args.input_file}")
    output_file = args.output_dir / (args.input_file.stem + ".trlc")
    record_count = convert(
        args.input_file, output_file, package=args.package, ref_package=args.ref_package
    )
    logging.info("%s -> %s  (%d record(s))", args.input_file, output_file, record_count)
