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
"""Load the normalized JSON of ``test_specification`` targets.

Each ``test_specification`` target parses and validates its RST file at build
time (see ``parse_test_spec.py``) and writes a ``<name>.test_spec.json``::

    {
      "ref": "//pkg:name",
      "aliases": ["//pkg:name.rst"],
      "goal": "...",
      "focus": "...",
      "steps": [{"precondition": "...", "action": "...", "expected": "..."}]
    }

``ref`` and ``aliases`` are labels relative to the repository of the spec, so
they do not change when the SEooC is built as an external dependency.

The coverage runners receive a manifest with one entry per line::

    <path to .test_spec.json>[<TAB><reference>]

``<reference>`` is the spec label as written in ``unit(test_specs = ...)``,
spelled in the repository of the unit (e.g. ``//pkg:name`` or
``@spec_repo//pkg:name``). Test cases of that unit use the same spelling, so it
resolves independently of canonical repository names. As a fallback, ``ref``
and ``aliases`` of the JSON resolve too, unless they are ambiguous.
"""

from __future__ import annotations

import json
from pathlib import Path

from test_case_coverage.compute_lock import TestSpec, normalize_label, spec_from_dict
from test_case_coverage.read_gtest_lobster import resolve_path


def load_test_spec_json(path: Path) -> tuple[TestSpec, list[str]]:
    """Read one ``.test_spec.json`` file and return the spec and its alias labels."""
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise ValueError(f"Cannot read test specification {path}: {exc}") from exc
    spec = spec_from_dict(data)
    return spec, [str(a) for a in data.get("aliases", [])]


def _add(table: dict[str, TestSpec | None], key: str, spec: TestSpec) -> None:
    """Add *spec* under *key*; a key claimed by two different specs becomes ambiguous (``None``)."""
    if key in table and table[key] != spec:
        table[key] = None
    else:
        table.setdefault(key, spec)


def load_test_spec_manifest(manifest_path: Path | None) -> dict[str, TestSpec | None]:
    """Return ``{normalized reference: TestSpec}`` for every entry of *manifest_path*.

    References given in the manifest (as written in ``test_specs``) take
    precedence. The ``ref`` and ``aliases`` of each JSON are added as fallback.
    A key that two different specs claim maps to ``None`` (ambiguous), so it
    does not resolve. ``None`` or an empty manifest yields an empty mapping.
    """
    if manifest_path is None:
        return {}

    explicit: dict[str, TestSpec | None] = {}
    fallback: dict[str, TestSpec | None] = {}
    cache: dict[str, tuple[TestSpec, list[str]]] = {}
    for line in manifest_path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        path, _, reference = line.partition("\t")
        path = path.strip()
        if path not in cache:
            cache[path] = load_test_spec_json(resolve_path(path))
        spec, aliases = cache[path]
        if reference.strip():
            _add(explicit, normalize_label(reference), spec)
        for label in [spec.ref, *aliases]:
            _add(fallback, normalize_label(label), spec)

    return {**fallback, **explicit}
