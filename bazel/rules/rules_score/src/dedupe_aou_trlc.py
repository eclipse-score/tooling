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
"""Deduplicate AoU/ReceivedAoU TRLC records reachable via more than one path.

``filter_forwarded_trlc.py`` deliberately keeps a chain-forwarded AoU's
identity (package + record name) identical to the original ``AoU`` record it
was retyped from, so a ``derived_from`` reference written once keeps
resolving no matter how many hops of forwarding it travels through. This is
exactly what makes a *diamond* dependency shape dangerous: if the same
original AoU is reachable both directly (from its owning
``assumptions_of_use``/``dependable_element``) and indirectly (via one or
more intermediate elements that chain-forward it), the same
``Package.RecordName`` identity ends up declared in more than one ``.trlc``
file that are simultaneously in scope for one TRLC check -- and TRLC's own
duplicate-definition check keys on ``(package, name)`` alone, not on
declared type, so it rejects this outright even though the two
declarations are, semantically, "the same AoU arriving twice."

This tool runs wherever such a set of TRLC files is merged together --
inside a ``dependable_element``'s own aggregation of what it received from
its ``deps`` (so its own re-exposed ``TrlcProviderInfo`` is never internally
inconsistent), and again in the shared ``requirements.bzl`` implementation
that merges ``TrlcProviderInfo`` across every ``deps`` entry of any
``feature_requirements``/``component_requirements``/
``assumed_system_requirements``/``assumptions_of_use`` target (so a target
that lists both an AoU's owner and a forwarder of that same AoU directly in
its own ``deps`` still resolves cleanly). It only ever considers
``ScoreReq.AoU`` and ``ScoreReq.ReceivedAoU`` records -- any other record
type colliding on the same identity is a genuine authoring error and is left
untouched so TRLC's own duplicate-definition check still catches it.

**A raw ``ScoreReq.AoU`` record is never re-exposed verbatim through any
``dependable_element``'s own ``TrlcProviderInfo``.** A target can always
resolve a ``derived_from`` reference to an AoU by depending directly on the
``assumptions_of_use`` target that authored it (the normal, fully-linked
consumption path, unaffected by any of this). What's retyped is only the
copy re-exposed *through a dependable_element's own aggregate
TrlcProviderInfo* -- whether it is the element's own directly-authored AoU
(first-hop exposure, see ``expose_own_aou_trlc.py``) or one it received and
is chain-forwarding further (see ``filter_forwarded_trlc.py``); either is
always retyped to ``ScoreReq.ReceivedAoU`` before being exposed that way.
Since a given ``package.name`` identity can only ever be legitimately
authored once (by its true owner, as ``ScoreReq.AoU``), any duplicate
declaration reachable through one or more dependable_elements'
``TrlcProviderInfo`` is therefore always the same original reached via a
different path -- never two independently-authored, unrelated AoUs that
coincidentally share a name (that general TRLC authoring risk exists for
every record type, not something introduced by AoU forwarding, and is
unaffected by this tool). For each ``Package.RecordName`` identity declared
more than once across the full set of input files, exactly one declaration
is kept and the rest are dropped (everything else in those files -- headers,
unrelated records -- is left untouched); an original ``ScoreReq.AoU`` is
preferred if one happens to be present (defensive only -- it should never
actually occur once ``expose_own_aou_trlc.py`` is wired in everywhere it
needs to be), otherwise the declaration from the lexicographically-first
input path is kept, purely for build-to-build determinism -- which specific
copy "wins" carries no semantic weight, since the identity, type, and
safety classification are the same either way and only the free-text
``justification`` may differ
between candidates.

This is a lightweight, regex/brace-matching based tool -- like
``rst_to_trlc.py``/``filter_forwarded_trlc.py`` -- not a full TRLC semantic
parser.
"""

from __future__ import annotations

import argparse
import logging
from pathlib import Path

from trlc_record_utils import extract_records, parse_package

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

logger = logging.getLogger(__name__)

_DEDUPE_TYPES = frozenset({"ScoreReq.AoU", "ScoreReq.ReceivedAoU"})


def dedupe_trlc_sources(
        sources: list[tuple[str, str]],
) -> tuple[list[str], dict[str, str]]:
    """Drop duplicate AoU/ReceivedAoU declarations across a set of .trlc files.

    Args:
        sources: List of ``(path, source_text)`` tuples, one per input file,
            in the order the caller wants ties broken by if no ``AoU``
            (non-forwarded) candidate is present for a given identity.

    Returns:
        A ``(filtered_sources, dropped_by_base_id)`` tuple:
        ``filtered_sources`` is order-aligned with ``sources``, each entry
        being that file's text with any losing duplicate record spans
        removed (files with no duplicates are returned byte-identical).
        ``dropped_by_base_id`` maps each ``Package.RecordName`` identity
        that had a duplicate to the path of the file whose declaration was
        kept (for logging/debugging).
    """
    parsed = [(path, parse_package(text, path), extract_records(text)) for path, text in sources]

    # base_id -> list of (file_index, record_index, record_type, path)
    occurrences: dict[str, list[tuple[int, int, str, str]]] = {}
    for file_index, (path, package, records) in enumerate(parsed):
        for record_index, (record_type, name, _text, _start, _end) in enumerate(records):
            if record_type not in _DEDUPE_TYPES:
                continue
            occurrences.setdefault(f"{package}.{name}", []).append((file_index, record_index, record_type, path))

    losers: set[tuple[int, int]] = set()
    kept_path_by_base_id: dict[str, str] = {}
    for base_id, occs in occurrences.items():
        if len(occs) <= 1:
            continue
        # Prefer an original (non-forwarded) AoU declaration -- defensive
        # only, see module docstring: this should never actually occur in
        # practice since a raw AoU record is never re-exposed verbatim
        # through any dependable_element's own TrlcProviderInfo. Otherwise
        # fall back to the lexicographically-first input path for
        # determinism.
        winner = min(occs, key=lambda o: (0 if o[2] == "ScoreReq.AoU" else 1, o[3]))
        kept_path_by_base_id[base_id] = winner[3]
        for occ in occs:
            if occ != winner:
                losers.add((occ[0], occ[1]))
        logger.info(
            "Duplicate AoU identity %s declared in %d files; keeping %s (%s)",
            base_id,
            len(occs),
            winner[3],
            winner[2],
        )

    filtered_sources: list[str] = []
    for file_index, (path, _package, records) in enumerate(parsed):
        _original_path, text = sources[file_index]
        spans_to_remove = sorted(
            (records[record_index][3], records[record_index][4])
            for record_index in range(len(records))
            if (file_index, record_index) in losers
        )
        if not spans_to_remove:
            filtered_sources.append(text)
            continue
        new_text = text
        for start, end in reversed(spans_to_remove):
            new_text = new_text[:start] + new_text[end:]
        filtered_sources.append(new_text)

    return filtered_sources, kept_path_by_base_id


def main() -> None:
    """Entry point for the AoU/ReceivedAoU TRLC deduplication tool."""
    parser = argparse.ArgumentParser(
        description="Deduplicate AoU/ReceivedAoU TRLC records reachable via more than one path.",
    )
    parser.add_argument(
        "--inputs",
        nargs="+",
        required=True,
        help="Input .trlc files (order-aligned with --outputs).",
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

    sources = [(path, Path(path).read_text(encoding="utf-8")) for path in args.inputs]
    filtered_sources, kept_path_by_base_id = dedupe_trlc_sources(sources)

    for output_path, filtered in zip(args.outputs, filtered_sources):
        out = Path(output_path)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(filtered, encoding="utf-8")

    logger.info("Resolved %d duplicate AoU identities across %d input files", len(kept_path_by_base_id), len(args.inputs))


if __name__ == "__main__":
    main()
