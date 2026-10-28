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

The coverage runners receive a manifest with one JSON path per line and look
up the ``test-spec`` reference of each test case by its normalized label.
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


def load_test_spec_manifest(manifest_path: Path | None) -> dict[str, TestSpec]:
    """Return ``{normalized label: TestSpec}`` for every JSON listed in *manifest_path*.

    Both the ``test_specification`` target label and its alias labels (the RST
    source file) are keys. ``None`` or an empty manifest yields an empty mapping.
    """
    specs: dict[str, TestSpec] = {}
    if manifest_path is None:
        return specs
    for line in manifest_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        spec, aliases = load_test_spec_json(resolve_path(line))
        for label in [spec.ref, *aliases]:
            specs[normalize_label(label)] = spec
    return specs
