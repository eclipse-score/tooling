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
"""Generate ``ScoreReq.FailureMode`` records from Sphinx-Needs FMEA needs.

The FMEA stays written as ``comp_saf_fmea`` needs (score_docs_as_code); this
tool turns each need of a ``needs.json`` export into a generated TRLC record,
so the fault trees, the safety-analysis page and the LOBSTER traceability of
rules_score work on it unchanged. The record name is the need ID; a fault tree
references it as ``<package>.<need ID>``.

Field mapping:

* ``title``          -> ``description``
* content            -> ``note``
* ``failure_effect`` -> ``failureeffect``
* ``fault_id``       -> ``guidewords`` (fault model IDs, see ``FAULT_ID_GUIDEWORDS``)
* ``violates``       -> ``interface``: a violated ``real_arc_int_op`` need names
  the ``ApiMethod`` whose signature contains ``<interface title>::<operation title>(``
* ``safety``         -> ``safety``; without it, the highest ``safety`` of the violated needs
* ``version``        -> ``version``

Needs with status ``invalid`` are skipped.
"""

from __future__ import annotations

import argparse
import json
import logging
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

# Fault model IDs of the S-CORE process (gd_guidl__fault_models) per guide word,
# as documented on the ScoreReq.Guideword enum.
FAULT_ID_GUIDEWORDS = {
    "MF_01_01": "LossOfFunction",
    "MF_01_06": "LossOfFunction",
    "EX_01_04": "LossOfFunction",
    "MF_01_04": "PartialFunction",
    "EX_01_06": "PartialFunction",
    "MF_01_05": "Corrupted",
    "MF_01_07": "UnintendedFunction",
    "CO_01_01": "TooEarly",
    "MF_01_03": "TooEarly",
    "CO_01_02": "TooLate",
    "MF_01_02": "TooLate",
    "EX_01_01": "Wrong",
    "EX_01_02": "DelayedFunction",
    "EX_01_03": "ExceedingFunction",
    "EX_01_05": "ArbitraryExecution",
}

# ScoreReq.Asil has no A and C.
_SAFETY_ASIL = {"QM": "QM", "ASIL_B": "B", "ASIL_D": "D"}
_ASIL_ORDER = ["QM", "B", "D"]

_RE_IDENTIFIER = re.compile(r"^[A-Za-z][A-Za-z0-9_]*$")
_RE_LINK = re.compile(r"^(?P<id>[^\[\]\s]+)\s*(?:\[.*\])?$")
_RE_PACKAGE = re.compile(r"^\s*package\s+([A-Za-z][A-Za-z0-9_]*)", re.MULTILINE)
_RE_API_METHOD = re.compile(r"ScoreReq\.ApiMethod\s+([A-Za-z][A-Za-z0-9_]*)\s*\{(.*?)\n\}", re.DOTALL)
_RE_SIGNATURES = re.compile(r"signatures\s*=\s*\[(.*?)\]", re.DOTALL)
_RE_STRING = re.compile(r'"((?:[^"\\]|\\.)*)"')


class ConversionError(Exception):
    """A need cannot be converted into a FailureMode record."""


@dataclass(frozen=True)
class ApiMethod:
    fqn: str
    signatures: tuple[str, ...]


def load_needs(path: str | Path) -> dict[str, dict[str, Any]]:
    """Return the needs of the current version of a Sphinx-Needs export."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    versions = data.get("versions") or {}
    current = data.get("current_version", "")
    if current in versions:
        return versions[current].get("needs", {})
    if len(versions) == 1:
        return next(iter(versions.values())).get("needs", {})
    raise ConversionError(f"{path}: cannot select a version, current_version {current!r} not in {sorted(versions)}")


def load_api_methods(paths: list[str]) -> list[ApiMethod]:
    """Collect the ``ApiMethod`` records of generated ``public_api.trlc`` files."""
    methods = []
    for path in paths:
        text = Path(path).read_text(encoding="utf-8")
        records = _RE_API_METHOD.findall(text)
        if not records:
            continue
        package = _RE_PACKAGE.search(text)
        if not package:
            raise ConversionError(f"{path}: ApiMethod records without a package declaration")
        for name, body in records:
            sig_list = _RE_SIGNATURES.search(body)
            signatures = tuple(s.strip("`") for s in _RE_STRING.findall(sig_list.group(1))) if sig_list else ()
            methods.append(ApiMethod(f"{package.group(1)}.{name}", signatures))
    return methods


def _values(need: dict[str, Any], field_name: str) -> list[str]:
    value = need.get(field_name)
    if value is None or value == "":
        return []
    if isinstance(value, list):
        return [str(v).strip() for v in value if str(v).strip()]
    return [v.strip() for v in str(value).split(",") if v.strip()]


def _link_ids(need: dict[str, Any], field_name: str) -> list[str]:
    ids = []
    for value in _values(need, field_name):
        match = _RE_LINK.match(value)
        if not match:
            raise ConversionError(f"{need['id']}: cannot parse {field_name} link {value!r}")
        ids.append(match.group("id"))
    return ids


def _string(text: str) -> str:
    """TRLC string literal on a single line."""
    return '"' + " ".join(text.split()).replace("\\", "\\\\").replace('"', '\\"') + '"'


def guidewords(need: dict[str, Any]) -> list[str]:
    """Map the fault model IDs of ``fault_id`` to guide words, keeping their order."""
    words: list[str] = []
    for fault_id in _values(need, "fault_id"):
        word = FAULT_ID_GUIDEWORDS.get(fault_id)
        if word is None:
            raise ConversionError(
                f"{need['id']}: unknown fault_id {fault_id!r}, expected one of {sorted(FAULT_ID_GUIDEWORDS)}"
            )
        if word not in words:
            words.append(word)
    if not words:
        raise ConversionError(f"{need['id']}: fault_id is missing")
    return words


def interface(need: dict[str, Any], needs: dict[str, dict[str, Any]], methods: list[ApiMethod]) -> list[str]:
    """``ApiMethod`` FQNs of the ``real_arc_int_op`` needs the FMEA violates."""
    fqns: list[str] = []
    for target_id in _link_ids(need, "violates"):
        operation = needs.get(target_id)
        if operation is None or operation.get("type") != "real_arc_int_op":
            continue
        for interface_id in _link_ids({"id": target_id, **operation}, "included_by"):
            interface_need = needs.get(interface_id)
            if interface_need is None:
                raise ConversionError(f"{need['id']}: interface {interface_id} of {target_id} is not in the needs")
            key = f"{interface_need.get('title', '').strip()}::{operation.get('title', '').strip()}("
            matches = sorted({m.fqn for m in methods if any(key in s for s in m.signatures)})
            if len(matches) != 1:
                found = f"several ApiMethods {matches}" if matches else "no ApiMethod"
                raise ConversionError(
                    f"{need['id']}: {found} of the public API declares {key!r} (violates {target_id})"
                )
            if matches[0] not in fqns:
                fqns.append(matches[0])
    return fqns


def safety(need: dict[str, Any], needs: dict[str, dict[str, Any]]) -> str:
    """ASIL of the need, else the highest ASIL of the needs it violates."""
    own = _values(need, "safety")
    values = own or [v for t in _link_ids(need, "violates") for v in _values(needs.get(t, {}), "safety")]
    if not values:
        raise ConversionError(f"{need['id']}: no safety, neither on the need nor on the needs it violates")
    asils = []
    for value in values:
        if value not in _SAFETY_ASIL:
            raise ConversionError(f"{need['id']}: safety {value!r} is not one of {sorted(_SAFETY_ASIL)}")
        asils.append(_SAFETY_ASIL[value])
    return max(asils, key=_ASIL_ORDER.index)


def convert_need(
    need: dict[str, Any], needs: dict[str, dict[str, Any]], methods: list[ApiMethod]
) -> tuple[str, list[str]]:
    """Return the TRLC record of one FMEA need and the packages it references."""
    if not _RE_IDENTIFIER.match(need["id"]):
        raise ConversionError(f"{need['id']}: the need ID is not a valid TRLC identifier")
    effect = str(need.get("failure_effect") or "").strip()
    if not effect:
        raise ConversionError(f"{need['id']}: failure_effect is missing")
    version = str(need.get("version") or "1")
    if not version.isdigit():
        raise ConversionError(f"{need['id']}: version {version!r} is not an integer")
    api = interface(need, needs, methods)

    location = f"{need.get('docname', '')}{need.get('doctype') or ''}"
    lines = [f"// {location}:{need['lineno']}" if location and need.get("lineno") else f"// {need['id']}"]
    lines.append(f"ScoreReq.FailureMode {need['id']} {{")
    lines.append(f"  description = {_string(str(need.get('title') or need['id']))}")
    content = str(need.get("content") or "").strip()
    if content:
        lines.append(f"  note = {_string(content)}")
    lines.append(f"  failureeffect = {_string(effect)}")
    lines.append("  guidewords = [" + ", ".join(f"ScoreReq.Guideword.{w}" for w in guidewords(need)) + "]")
    if api:
        lines.append("  interface = [" + ", ".join(api) + "]")
    lines.append(f"  safety = ScoreReq.Asil.{safety(need, needs)}")
    lines.append(f"  version = {int(version)}")
    lines.append("}")
    return "\n".join(lines), [fqn.split(".", 1)[0] for fqn in api]


def convert(
    needs_files: list[str], api_files: list[str], package: str, need_type: str = "comp_saf_fmea"
) -> tuple[str, int]:
    """Render the TRLC file of all FMEA needs; returns the text and the record count."""
    if not _RE_IDENTIFIER.match(package):
        raise ConversionError(f"package {package!r} is not a valid TRLC identifier")
    needs: dict[str, dict[str, Any]] = {}
    for path in needs_files:
        for need_id, need in load_needs(path).items():
            if need_id in needs:
                raise ConversionError(f"need {need_id} is defined in more than one needs.json")
            needs[need_id] = {"id": need_id, **need}
    methods = load_api_methods(api_files)

    records: list[str] = []
    imports: list[str] = []
    for need_id in sorted(needs):
        need = needs[need_id]
        if need.get("type") != need_type:
            continue
        if need.get("status") == "invalid":
            logger.info("Skipping %s with status invalid", need_id)
            continue
        record, packages = convert_need(need, needs, methods)
        records.append(record)
        imports.extend(p for p in packages if p not in imports)

    header = [f"// GENERATED by fmea_needs_to_trlc from the {need_type} needs -- do not edit", f"package {package}", ""]
    if not records:
        return "\n".join(header), 0
    header += ["import ScoreReq"] + [f"import {p}" for p in sorted(imports)] + [""]
    return "\n".join(header + ["\n\n".join(records), ""]), len(records)


def main(argv: list[str] | None = None) -> int:
    """Entry point of the generator."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("needs", nargs="+", help="needs.json files with the FMEA needs")
    parser.add_argument("--package", required=True, help="TRLC package of the generated records")
    parser.add_argument(
        "--api-trlc", action="append", default=[], help="TRLC file holding ApiMethod records (repeatable)"
    )
    parser.add_argument("--need-type", default="comp_saf_fmea", help="need type to convert")
    parser.add_argument("--out", required=True, help="output .trlc file")
    parser.add_argument("--log-level", choices=sorted(_LEVEL_MAP), default="warn", dest="log_level")
    args = parser.parse_args(argv)
    logging.basicConfig(level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s")

    try:
        text, count = convert(args.needs, args.api_trlc, args.package, args.need_type)
    except ConversionError as e:
        logger.error("%s", e)
        return 1
    if not count:
        logger.warning("no %s need found in %s", args.need_type, ", ".join(args.needs))
    Path(args.out).write_text(text, encoding="utf-8")
    logger.info("Wrote %d FailureMode record(s) to %s", count, args.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
