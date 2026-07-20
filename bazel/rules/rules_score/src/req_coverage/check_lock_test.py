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
from req_coverage.check_lock import compare_lock_files, validate_specs
from req_coverage.compute_lock import LockFile, RequirementEntry, TestCase


def _lock(*reqs):
    return LockFile(schema_version=2, requirements=list(reqs))


def _req(req_id, *test_cases, version="1"):
    return RequirementEntry(id=req_id, version=version, test_cases=list(test_cases))


def _tc(uid="Suite:TestName", given="g", when="w", then="t"):
    return TestCase(uid=uid, given=given, when=when, then=then)


def test_identical_locks_pass():
    lock = _lock(_req("P.R", _tc()))
    ok, diff = compare_lock_files(lock, lock)
    assert ok
    assert diff == []


def test_test_case_added_fails():
    committed = _lock(_req("P.R", _tc(uid="Suite:A")))
    computed = _lock(_req("P.R", _tc(uid="Suite:A"), _tc(uid="Suite:B")))
    ok, diff = compare_lock_files(committed, computed)
    assert not ok
    assert any("Suite:B" in line and "added" in line for line in diff)


def test_test_case_removed_fails():
    committed = _lock(_req("P.R", _tc(uid="Suite:A"), _tc(uid="Suite:B")))
    computed = _lock(_req("P.R", _tc(uid="Suite:A")))
    ok, diff = compare_lock_files(committed, computed)
    assert not ok
    assert any("Suite:B" in line and "removed" in line for line in diff)


def test_spec_changed_fails():
    tc_old = TestCase(uid="Suite:A", given="g", when="w", then="old")
    tc_new = TestCase(uid="Suite:A", given="g", when="w", then="new")
    committed = _lock(_req("P.R", tc_old))
    computed = _lock(_req("P.R", tc_new))
    ok, diff = compare_lock_files(committed, computed)
    assert not ok
    assert any("Suite:A" in line and "changed" in line for line in diff)


def test_requirement_added_fails():
    committed = _lock(_req("P.A"))
    computed = _lock(_req("P.A"), _req("P.B"))
    ok, diff = compare_lock_files(committed, computed)
    assert not ok
    assert any("P.B" in line and "added" in line for line in diff)


def test_requirement_removed_fails():
    committed = _lock(_req("P.A"), _req("P.B"))
    computed = _lock(_req("P.A"))
    ok, diff = compare_lock_files(committed, computed)
    assert not ok
    assert any("P.B" in line and "removed" in line for line in diff)


def test_version_changed_fails():
    committed = _lock(_req("P.R", _tc(), version="1"))
    computed = _lock(_req("P.R", _tc(), version="2"))
    ok, diff = compare_lock_files(committed, computed)
    assert not ok
    assert any("version changed" in line and "P.R" in line for line in diff)


def test_version_unchanged_passes():
    committed = _lock(_req("P.R", _tc(), version="3"))
    computed = _lock(_req("P.R", _tc(), version="3"))
    ok, diff = compare_lock_files(committed, computed)
    assert ok


def test_description_change_does_not_fail():
    """Description is display-only — a change must not affect the drift verdict."""
    committed = _lock(
        RequirementEntry(id="P.R", version="1", description="old", test_cases=[_tc()])
    )
    computed = _lock(
        RequirementEntry(id="P.R", version="1", description="new", test_cases=[_tc()])
    )
    ok, diff = compare_lock_files(committed, computed)
    assert ok


def test_empty_vs_empty_passes():
    ok, diff = compare_lock_files(_lock(), _lock())
    assert ok
    assert diff == []


# ---------------------------------------------------------------------------
# validate_specs tests
# ---------------------------------------------------------------------------


def test_validate_specs_passes_with_spec():
    lock = _lock(_req("P.R", _tc()))
    ok, issues = validate_specs(lock)
    assert ok
    assert issues == []


def test_validate_specs_fails_with_empty_spec():
    lock = _lock(_req("P.R", _tc(given="", when="", then="")))
    ok, issues = validate_specs(lock)
    assert not ok
    assert any("Suite:TestName" in line for line in issues)
    assert any("P.R" in line for line in issues)


def test_validate_specs_fails_with_whitespace_only_spec():
    lock = _lock(_req("P.R", _tc(given="  ", when="  ", then="  ")))
    ok, issues = validate_specs(lock)
    assert not ok


def test_validate_specs_reports_all_missing():
    lock = _lock(
        _req("P.A", _tc(uid="Suite:A", given="", when="", then="")),
        _req("P.B", _tc(uid="Suite:B", given="", when="", then="")),
    )
    ok, issues = validate_specs(lock)
    assert not ok
    assert len(issues) == 2


def test_validate_specs_empty_lock_passes():
    ok, issues = validate_specs(_lock())
    assert ok
    assert issues == []


if __name__ == "__main__":
    import sys
    import pytest

    sys.exit(pytest.main([__file__, "-v"]))
