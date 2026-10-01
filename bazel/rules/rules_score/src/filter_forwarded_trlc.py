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
"""Filter received AoU TRLC source records for chain-forwarding.

Companion to ``aou_forwarding_to_lobster.py``, operating on the raw ``.trlc``
requirement *source* files instead of already-extracted lobster JSON. This is
what lets a dependable_element expose the TRLC records of the AoUs it
chain-forwards as part of its own ``TrlcProviderInfo``, so that a downstream
``component_requirements``/``feature_requirements``/... target can list the
dependable_element in its own ``deps`` and resolve a
``derived_from = [Pkg.SomeAoU@1]`` cross-reference against it.

Reads a chain-forwarding YAML file (the same ``aou_forwarding.yaml`` format
used by ``aou_forwarding_to_lobster.py``) and, for each ``(input, output)``
``.trlc`` file pair, writes a filtered copy of the input file: the original
``package``/``import`` header is preserved verbatim (so the output remains
syntactically valid TRLC even if zero records match), but only the top-level
record bodies whose id matches an entry in the forwarding YAML are copied
across -- retyped from ``ScoreReq.AoU`` (or, for a multi-hop chain, an
already-``ScoreReq.ReceivedAoU``) to ``ScoreReq.ReceivedAoU``, with a
``justification`` field injected (carried over from the YAML entry). Every
other record body in the file is dropped. Retyping (rather than copying the
original ``AoU`` record verbatim) is deliberate: a raw duplicate ``AoU``
would be indistinguishable from a second, independently authored assumption
that itself needs full control-measure/safety-analysis linkage, whereas
``ReceivedAoU`` is recognizably a pass-through forwarding placeholder that
shares its identity (package + record name) with the original so
``derived_from`` references keep working unchanged across the whole
forwarding chain.

Every entry in the forwarding YAML must match at least one record across all
input files; an entry that matches nothing (typo, or an AoU this element
never actually received) is a hard error -- mirroring the same validation
``aou_forwarding_to_lobster.py`` already performs against received lobster
items, so a misconfigured ``aou_forwarding.yaml`` fails loudly rather than
silently under-forwarding at the TRLC level.

This is a lightweight, regex/brace-matching based tool -- like
``rst_to_trlc.py`` -- not a full TRLC semantic parser. It only needs to
recognize top-level ``<Package.Type> <Name> { ... }`` record blocks, since
that is the only shape ``score_requirements_rule``-generated (and
hand-authored, following the same convention) TRLC requirement files use.
"""

from __future__ import annotations

import argparse
import logging
from pathlib import Path

from aou_forwarding_to_lobster import parse_forwarding_yaml
from trlc_record_utils import base_id as _base_id
from trlc_record_utils import extract_records, parse_package
from trlc_record_utils import retype_as_received_aou as _retype_as_received_aou

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

logger = logging.getLogger(__name__)


def filter_trlc_source(
    source: str,
    forwarding_entries: list[dict[str, str]],
    path: str = "<string>",
) -> tuple[str, set[str]]:
    """Filter a .trlc source string down to only the forwarded records.

    Args:
        source: Full text of the received .trlc file.
        forwarding_entries: Parsed YAML entries with 'aou_id' and
            'justification' fields (see
            ``aou_forwarding_to_lobster.parse_forwarding_yaml``). TRLC record
            identity has no ``@version`` component (unlike the lobster tag),
            so any ``@version`` suffix on an entry's ``aou_id`` is ignored
            here -- matching is by ``Package.RecordName`` alone.
        path: Path to the file (for error messages only).

    Returns:
        A ``(filtered_source, matched_base_ids)`` tuple: the filtered .trlc
        source (original header unchanged, followed by only the matched
        records -- retyped to ``ReceivedAoU`` with their ``justification``
        field injected, in their original order), and the set of
        ``Package.RecordName`` base ids that were actually matched in this
        file (for the caller to accumulate across all input files and
        validate every YAML entry was matched at least once).
    """
    package = parse_package(source, path)
    records = extract_records(source)

    justification_by_base_id = {_base_id(e["aou_id"]): e["justification"] for e in forwarding_entries}

    header_end = records[0][3] if records else len(source)
    header = source[:header_end]

    kept: list[str] = []
    matched_base_ids: set[str] = set()
    for record_type, name, text, _, _ in records:
        base_id = f"{package}.{name}"
        if base_id in justification_by_base_id:
            kept.append(_retype_as_received_aou(text, justification_by_base_id[base_id]))
            matched_base_ids.add(base_id)

    body = "\n\n".join(kept)
    if body:
        filtered = header.rstrip("\n") + "\n\n" + body + "\n"
    else:
        filtered = header
    return filtered, matched_base_ids


def check_all_entries_matched(wanted_base_ids: set[str], all_matched_base_ids: set[str]) -> None:
    """Fail loudly if any forwarding YAML entry matched no record.

    Mirrors ``aou_forwarding_to_lobster.py``'s ``_match_forwarded_entries``
    behavior: a forwarding YAML entry that never matched any received AoU
    TRLC record (typo, or an AoU this element never actually received) is a
    configuration error, not something to silently ignore.

    Args:
        wanted_base_ids: All ``Package.RecordName`` base ids listed in the
            forwarding YAML.
        all_matched_base_ids: All base ids actually matched across every
            processed input file.

    Raises:
        SystemExit: If any entry in ``wanted_base_ids`` was never matched.
    """
    unmatched = sorted(wanted_base_ids - all_matched_base_ids)
    if not unmatched:
        return
    available = ", ".join(sorted(all_matched_base_ids)) if all_matched_base_ids else "(none)"
    raise SystemExit(
        "aou_forwarding.yaml entr%s not found in received AoU TRLC sources: %s. Available IDs: %s"
        % ("y" if len(unmatched) == 1 else "ies", ", ".join(unmatched), available)
    )


def main() -> None:
    """Entry point for the TRLC AoU chain-forwarding filter tool."""
    parser = argparse.ArgumentParser(description="Filter received AoU TRLC source records for chain-forwarding.")
    parser.add_argument(
        "--yaml",
        required=True,
        help="Path to the aou_forwarding.yaml file listing AoU IDs to further-forward.",
    )
    parser.add_argument(
        "--inputs",
        nargs="+",
        required=True,
        help="Received .trlc files (order-aligned with --outputs).",
    )
    parser.add_argument(
        "--outputs",
        nargs="+",
        required=True,
        help="Output .trlc file paths, one per --inputs entry, same order.",
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

    if len(args.inputs) != len(args.outputs):
        raise SystemExit(
            f"--inputs has {len(args.inputs)} entries but --outputs has {len(args.outputs)}; "
            "they must be order-aligned and the same length."
        )

    forwarding_entries = parse_forwarding_yaml(args.yaml)

    all_matched_base_ids: set[str] = set()
    for input_path, output_path in zip(args.inputs, args.outputs):
        source = Path(input_path).read_text(encoding="utf-8")
        filtered, matched_base_ids = filter_trlc_source(source, forwarding_entries, input_path)
        all_matched_base_ids |= matched_base_ids
        out = Path(output_path)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(filtered, encoding="utf-8")
        logger.info("Wrote filtered TRLC %s -> %s", input_path, output_path)

    wanted_base_ids = {_base_id(e["aou_id"]) for e in forwarding_entries}
    check_all_entries_matched(wanted_base_ids, all_matched_base_ids)

    logger.info(
        "Matched %d/%d forwarding entries to received AoU TRLC records",
        len(wanted_base_ids & all_matched_base_ids),
        len(wanted_base_ids),
    )


if __name__ == "__main__":
    main()
