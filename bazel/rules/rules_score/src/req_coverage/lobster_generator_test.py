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
import json
from pathlib import Path

from req_coverage.compute_lock import LockFile, RequirementEntry, TestCase
from req_coverage.lobster_generator import generate_lobster


def _make_lock(*reqs):
    return LockFile(schema_version=3, requirements=list(reqs))


def _req(req_id, *tcs, version="1"):
    return RequirementEntry(id=req_id, version=version, test_cases=list(tcs))


def _tc(uid="Suite:A", given="g", when="w", then="t"):
    return TestCase(uid=uid, given=given, when=when, then=then)


def test_generates_valid_lobster_json(tmp_path):
    computed = _make_lock(_req("P.R", _tc()))
    committed = _make_lock(_req("P.R", _tc()))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, committed, "//pkg:cov", out)
    data = json.loads(out.read_text())
    assert data["schema"] == "lobster-act-trace"
    assert data["version"] == 3
    assert data["generator"] == "req_coverage"
    assert len(data["data"]) == 1


def test_one_activity_per_requirement(tmp_path):
    """Each requirement produces exactly one Activity, even if the same test covers both."""
    tc = _tc(uid="S:T")
    computed = _make_lock(_req("P.A", tc), _req("P.B", tc))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, None, "//pkg:cov", out)
    data = json.loads(out.read_text())["data"]
    tags = [item["tag"] for item in data]
    assert len(tags) == 2
    assert set(tags) == {"req_coverage P.A", "req_coverage P.B"}


def test_covered_test_has_status_ok(tmp_path):
    computed = _make_lock(_req("P.R", _tc()))
    committed = _make_lock(_req("P.R", _tc()))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, committed, "//pkg:cov", out)
    item = json.loads(out.read_text())["data"][0]
    assert item["status"] == "ok"


def test_uncovered_test_has_drift_message(tmp_path):
    tc_new = TestCase(uid="Suite:A", given="g", when="w", then="new")
    tc_old = TestCase(uid="Suite:A", given="g", when="w", then="old")
    computed = _make_lock(_req("P.R", tc_new))
    committed = _make_lock(_req("P.R", tc_old))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, committed, "//pkg:cov", out)
    item = json.loads(out.read_text())["data"][0]
    assert item["status"] == "fail"
    assert any("Requirement Coverage not confirmed" in m for m in item["messages"])


def test_new_test_not_in_committed_has_status_fail(tmp_path):
    computed = _make_lock(_req("P.R", _tc()))
    committed = _make_lock(_req("P.R"))  # no test cases
    out = tmp_path / "out.lobster"
    generate_lobster(computed, committed, "//pkg:cov", out)
    item = json.loads(out.read_text())["data"][0]
    assert item["status"] == "fail"


def test_emitted_even_with_none_committed(tmp_path):
    computed = _make_lock(_req("P.R", _tc()))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, None, "//pkg:cov", out)
    assert out.exists()
    data = json.loads(out.read_text())
    assert data["data"][0]["status"] == "fail"


def test_tracing_ref_points_to_requirement(tmp_path):
    computed = _make_lock(_req("P.R", _tc("S.A")))
    committed = _make_lock(_req("P.R", _tc("S.A")))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, committed, "//pkg:cov", out)
    item = json.loads(out.read_text())["data"][0]
    assert "req P.R" in item["refs"]


def test_text_contains_gwt(tmp_path):
    computed = _make_lock(_req("P.R", _tc(given="g1", when="w1", then="t1")))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, None, "//pkg:cov", out)
    item = json.loads(out.read_text())["data"][0]
    assert "g1" in item.get("text", "")
    assert "w1" in item.get("text", "")
    assert "t1" in item.get("text", "")


def test_empty_gwt_omits_text(tmp_path):
    computed = _make_lock(_req("P.R", _tc(given="", when="", then="")))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, None, "//pkg:cov", out)
    item = json.loads(out.read_text())["data"][0]
    assert item.get("text") is None


def test_requirement_with_no_test_cases_produces_no_activity(tmp_path):
    computed = _make_lock(_req("P.R"))  # no test cases
    out = tmp_path / "out.lobster"
    generate_lobster(computed, None, "//pkg:cov", out)
    assert json.loads(out.read_text())["data"] == []


def test_text_contains_all_test_gwt_blocks(tmp_path):
    computed = _make_lock(
        _req(
            "P.R",
            _tc(uid="S:A", given="g1", when="w1", then="t1"),
            _tc(uid="S:B", given="g2", when="w2", then="t2"),
        )
    )
    out = tmp_path / "out.lobster"
    generate_lobster(computed, None, "//pkg:cov", out)
    text = json.loads(out.read_text())["data"][0].get("text", "")
    assert "g1" in text and "g2" in text


def test_removed_test_case_causes_fail(tmp_path):
    """Committed lock has 2 tests; computed has only 1 (tracing tag removed) → PARTIAL."""
    tc_a = _tc(uid="S:A")
    tc_b = _tc(uid="S:B")
    computed = _make_lock(_req("P.R", tc_a))  # S:B disappeared
    committed = _make_lock(_req("P.R", tc_a, tc_b))  # S:B was committed
    out = tmp_path / "out.lobster"
    generate_lobster(computed, committed, "//pkg:cov", out)
    item = json.loads(out.read_text())["data"][0]
    assert item["status"] == "fail"
    assert any("requirement coverage not confirmed" in m for m in item["messages"])


def test_version_changed_causes_fail(tmp_path):
    """Requirement version bump in computed but committed lock still has old version → fail."""
    tc = _tc()
    computed = _make_lock(_req("P.R", tc, version="2"))
    committed = _make_lock(_req("P.R", tc, version="1"))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, committed, "//pkg:cov", out)
    item = json.loads(out.read_text())["data"][0]
    assert item["status"] == "fail"
    assert any("requirement coverage not confirmed" in m for m in item["messages"])
    assert any("Requirement Version updated" in m for m in item["messages"])


def test_same_version_and_gwt_is_ok(tmp_path):
    tc = _tc()
    lock = _make_lock(_req("P.R", tc, version="3"))
    out = tmp_path / "out.lobster"
    generate_lobster(lock, lock, "//pkg:cov", out)
    assert json.loads(out.read_text())["data"][0]["status"] == "ok"

    """Activity is ok only when every test case in the req matches committed."""
    tc_ok = _tc(uid="S:A", given="g", when="w", then="t")
    tc_drift = TestCase(uid="S:B", given="g", when="w", then="new")
    tc_old = TestCase(uid="S:B", given="g", when="w", then="old")
    computed = _make_lock(_req("P.R", tc_ok, tc_drift))
    committed = _make_lock(_req("P.R", tc_ok, tc_old))
    out = tmp_path / "out.lobster"
    generate_lobster(computed, committed, "//pkg:cov", out)
    item = json.loads(out.read_text())["data"][0]
    assert item["status"] == "fail"


if __name__ == "__main__":
    import sys
    import pytest

    sys.exit(pytest.main([__file__, "-v"]))
