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
"""Retype a dependable_element's own AoU TRLC records for first-hop exposure.

Companion to ``filter_forwarded_trlc.py``. A dependable_element's own
``assumptions_of_use`` targets author their AoUs as plain ``ScoreReq.AoU``
records. A downstream target can always resolve a ``derived_from``
reference to one of these by depending directly on that
``assumptions_of_use`` target -- that is the normal, fully-linked way to
consume an AoU, and this tool has no effect on it. What must never happen
is that raw ``AoU`` record being re-exposed, verbatim, through the
dependable_element's own aggregate ``TrlcProviderInfo`` -- used by anything
that depends on the dependable_element label instead, precisely so it does
not need to know the AoU's true owner. Doing so verbatim would let a
downstream target's TRLC compilation exercise the exact same "dangling
record with no linkage in this scope" problem that
``filter_forwarded_trlc.py`` was written to avoid for chain-forwarded
records.

So every AoU a dependable_element re-exposes through its own
``TrlcProviderInfo`` -- whether it is one of its own (first-hop exposure,
this tool) or one it received and is chain-forwarding further
(``filter_forwarded_trlc.py``) -- is retyped to ``ScoreReq.ReceivedAoU``.
Internally, within the ``assumptions_of_use`` target's own compilation (and
for any consumer depending on it directly), the record stays
``ScoreReq.AoU``; only the copy re-exposed via the owning
dependable_element's ``TrlcProviderInfo`` is rewritten.

Unlike ``filter_forwarded_trlc.py``, there is no YAML-based selection here:
a dependable_element's own AoUs are unconditionally exposed in full through
its own ``TrlcProviderInfo`` (only chain-forwarding -- re-exposing AoUs
*received from* a dependency -- is gated by ``aou_forwarding.yaml``), so
every record in every input file is retyped and kept, using a fixed,
generic ``justification`` (there is no per-AoU forwarding-YAML entry to
source per-record justification text from, since this isn't a forwarding
decision -- it's the element making its own AoU visible for downstream
cross-referencing).

This is a lightweight, regex/brace-matching based tool -- like
``rst_to_trlc.py``/``filter_forwarded_trlc.py`` -- not a full TRLC semantic
parser.
"""

from __future__ import annotations

import argparse
import logging
from pathlib import Path

from trlc_record_utils import extract_records, retype_as_received_aou

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

logger = logging.getLogger(__name__)

DEFAULT_JUSTIFICATION = "Directly owned by this dependable_element; exposed for downstream requirement traceability."


def expose_own_aou_source(source: str, justification: str = DEFAULT_JUSTIFICATION) -> str:
    """Retype every top-level AoU record in a .trlc source string.

    Args:
        source: Full text of the owning ``assumptions_of_use`` target's
            .trlc file.
        justification: The ``justification`` field text to embed in every
            retyped record.

    Returns:
        The rewritten .trlc source: original header preserved verbatim,
        every record retyped to ``ScoreReq.ReceivedAoU`` with
        ``justification`` injected, in original order.
    """
    records = extract_records(source)
    header_end = records[0][3] if records else len(source)
    header = source[:header_end]

    kept = [retype_as_received_aou(text, justification) for _record_type, _name, text, _start, _end in records]

    body = "\n\n".join(kept)
    if body:
        return header.rstrip("\n") + "\n\n" + body + "\n"
    return header


def main() -> None:
    """Entry point for the own-AoU first-hop exposure retyping tool."""
    parser = argparse.ArgumentParser(
        description="Retype a dependable_element's own AoU TRLC records for first-hop external exposure.",
    )
    parser.add_argument(
        "--inputs",
        nargs="+",
        required=True,
        help="Owning assumptions_of_use .trlc files (order-aligned with --outputs).",
    )
    parser.add_argument(
        "--outputs",
        nargs="+",
        required=True,
        help="Output .trlc file paths, one per --inputs entry, same order.",
    )
    parser.add_argument(
        "--justification",
        default=DEFAULT_JUSTIFICATION,
        help="Justification text to embed in every retyped record (default: a generic own-AoU exposure notice).",
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

    for input_path, output_path in zip(args.inputs, args.outputs):
        source = Path(input_path).read_text(encoding="utf-8")
        exposed = expose_own_aou_source(source, args.justification)
        out = Path(output_path)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(exposed, encoding="utf-8")
        logger.info("Wrote own-AoU exposure TRLC %s -> %s", input_path, output_path)


if __name__ == "__main__":
    main()
