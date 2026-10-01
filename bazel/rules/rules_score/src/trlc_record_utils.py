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
"""Lightweight, regex/brace-matching helpers for reading ``.trlc`` source text.

Shared by ``filter_forwarded_trlc.py`` (selects+retypes chain-forwarded AoU
records), ``expose_own_aou_trlc.py`` (retypes a dependable_element's own
directly-authored AoU records for first-hop external exposure), and
``dedupe_aou_trlc.py`` (drops duplicate AoU/ReceivedAoU identities that
reach the same TRLC check via more than one path). None of these tools is a
full TRLC semantic parser -- they only need to recognize top-level
``<Package.Type> <Name> { ... }`` record blocks, since that is the only shape
``score_requirements_rule``-generated (and hand-authored, following the same
convention) TRLC requirement files use.
"""

from __future__ import annotations

import re

_RE_PACKAGE = re.compile(r"^package\s+(\S+)\s*$", re.MULTILINE)
# Matches the start of a top-level record: "<Package.Type> <Name> {" at the
# beginning of a line (column 0), mirroring the output shape of
# rst_to_trlc.py's render_trlc() and every hand-authored .trlc fixture in
# this repository.
_RE_RECORD_START = re.compile(r"^([\w.]+)\s+([\w]+)\s*\{", re.MULTILINE)


def parse_package(source: str, path: str) -> str:
    """Extract the ``package NAME`` declaration from a .trlc source string.

    Args:
        source: Full text of the .trlc file.
        path: Path to the file (for error messages only).

    Returns:
        The declared package name.

    Raises:
        SystemExit: If no ``package`` statement is found.
    """
    m = _RE_PACKAGE.search(source)
    if not m:
        raise SystemExit(f"TRLC file {path} has no 'package NAME' declaration.")
    return m.group(1)


def extract_records(source: str) -> list[tuple[str, str, str, int, int]]:
    """Find all top-level record blocks in a .trlc source string.

    Args:
        source: Full text of the .trlc file.

    Returns:
        List of (record_type, record_name, record_text, start_offset,
        end_offset) tuples, in file order. ``record_type`` is the fully
        qualified type token (e.g. ``ScoreReq.AoU``). ``record_text`` spans
        from the start of the record's first line
        (``<Package.Type> <Name> {``) through its matching closing brace,
        inclusive.

    Raises:
        SystemExit: If a record's braces are unbalanced.
    """
    records: list[tuple[str, str, str, int, int]] = []
    for m in _RE_RECORD_START.finditer(source):
        record_type = m.group(1)
        name = m.group(2)
        start = m.start()
        # Find the matching closing brace by counting braces from the
        # record's opening brace onward. Requirement record bodies in this
        # codebase are flat attribute lists (no nested braces), but brace
        # counting keeps this correct even if a value happens to contain one.
        depth = 0
        end = None
        for i in range(m.end() - 1, len(source)):
            if source[i] == "{":
                depth += 1
            elif source[i] == "}":
                depth -= 1
                if depth == 0:
                    end = i + 1
                    break
        if end is None:
            raise SystemExit(f"Unterminated record '{name}' (unbalanced braces).")
        records.append((record_type, name, source[start:end], start, end))
    return records


def base_id(aou_id: str) -> str:
    """Strip an optional '@version' suffix from a forwarding YAML aou_id."""
    return aou_id.split("@", 1)[0]


RECEIVED_AOU_TYPE = "ScoreReq.ReceivedAoU"
# Matches an existing "justification = "...""" field line (with escaped
# quotes/backslashes inside the string handled), so a record that is already
# ReceivedAoU (an own-AoU first-hop exposure, or an earlier forwarding hop)
# has its previous justification removed before a new one is injected --
# otherwise TRLC rejects the record for assigning the same component twice.
_RE_JUSTIFICATION_FIELD = re.compile(r'^[ \t]*justification\s*=\s*"(?:[^"\\]|\\.)*"[ \t]*\n?', re.MULTILINE)


def escape_trlc_string(text: str) -> str:
    """Escape a string for use inside a TRLC double-quoted literal."""
    return text.replace("\\", "\\\\").replace('"', '\\"')


def retype_as_received_aou(record_text: str, justification: str) -> str:
    """Rewrite a record's type to ``ScoreReq.ReceivedAoU`` and inject ``justification``.

    Used both when chain-forwarding an already-received AoU/ReceivedAoU
    (``filter_forwarded_trlc.py``) and when first exposing a
    dependable_element's own directly-authored ``AoU`` records through the
    dependable_element's own ``TrlcProviderInfo``
    (``expose_own_aou_trlc.py``) -- in both cases the *retyped* record is
    only what a `dependable_element` re-exposes to a consumer that depends
    on its label instead of the true owner. A target that depends directly
    on the `assumptions_of_use` target that authored the AoU still resolves
    against the true, unmodified ``AoU`` record -- that consumption path is
    unaffected by this retyping and remains the normal, fully-linked way to
    reference an AoU. The retyped placeholder shares its identity (package
    + record name) with the original so ``derived_from`` references keep
    working unchanged regardless of which of the two paths resolved them.

    If the record already carries a ``justification`` field (it is already
    ``ReceivedAoU`` -- own-AoU first-hop exposure, or an earlier forwarding
    hop), that existing field is replaced rather than duplicated: TRLC
    rejects a record that assigns the same component twice.

    Args:
        record_text: The original record text, starting with
            ``<Package.Type> <Name> {`` (see ``extract_records``).
        justification: The forwarding/exposure justification text to embed
            as the record's ``justification`` field.

    Returns:
        The rewritten record text, same body otherwise.
    """
    first_newline = record_text.find("\n")
    first_line = record_text[:first_newline] if first_newline != -1 else record_text
    rest = record_text[first_newline:] if first_newline != -1 else ""
    rest = _RE_JUSTIFICATION_FIELD.sub("", rest, count=1)

    new_first_line = _RE_RECORD_START.sub(
        lambda m: f"{RECEIVED_AOU_TYPE} {m.group(2)} {{",
        first_line,
        count=1,
    )
    return f'{new_first_line}\n    justification = "{escape_trlc_string(justification)}"{rest}'
