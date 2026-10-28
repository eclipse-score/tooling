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
"""Tests for test_spec.py (loading test_specification JSON manifests)."""

import json

import pytest

from test_case_coverage.compute_lock import TestSpec, TestStep, normalize_label
from test_case_coverage.test_spec import load_test_spec_manifest


def _write_spec(path, ref="//pkg:spec", aliases=("//pkg:spec.rst",)):
    path.write_text(
        json.dumps(
            {
                "ref": ref,
                "aliases": list(aliases),
                "goal": "goal",
                "focus": "",
                "steps": [{"precondition": "", "action": "a", "expected": "e"}],
            }
        ),
        encoding="utf-8",
    )


@pytest.mark.parametrize(
    "raw, expected",
    [
        ("//pkg:spec", "//pkg:spec"),
        ("  //pkg:spec ", "//pkg:spec"),
        ("@@//pkg:spec", "//pkg:spec"),
        ("@//pkg:spec", "//pkg:spec"),
        ("//pkg/spec", "//pkg/spec:spec"),
        ("@other//pkg:spec", "@other//pkg:spec"),
    ],
)
def test_normalize_label(raw, expected):
    assert normalize_label(raw) == expected


def test_manifest_maps_ref_and_aliases(tmp_path):
    spec_json = tmp_path / "spec.test_spec.json"
    _write_spec(spec_json)
    manifest = tmp_path / "manifest.txt"
    manifest.write_text(f"{spec_json}\n\n", encoding="utf-8")

    specs = load_test_spec_manifest(manifest)

    expected = TestSpec(ref="//pkg:spec", goal="goal", steps=(TestStep(action="a", expected="e"),))
    assert specs == {"//pkg:spec": expected, "//pkg:spec.rst": expected}


def test_no_manifest_yields_no_specs():
    assert load_test_spec_manifest(None) == {}


def test_unreadable_spec_json_raises(tmp_path):
    manifest = tmp_path / "manifest.txt"
    manifest.write_text(f"{tmp_path / 'missing.json'}\n", encoding="utf-8")
    with pytest.raises(ValueError, match="Cannot read test specification"):
        load_test_spec_manifest(manifest)


if __name__ == "__main__":
    import sys

    sys.exit(pytest.main([__file__, "-v"]))
