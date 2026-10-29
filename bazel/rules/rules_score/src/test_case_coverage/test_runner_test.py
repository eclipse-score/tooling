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

import pytest

from test_case_coverage.compute_lock import LockFile, RequirementEntry, TestCase, TestSpec, TestStep, serialize
from test_case_coverage.test_runner import main as runner_main
from test_case_coverage.conftest import write_gtest_lobster, write_req_lobster, write_test_spec_manifest

_SPEC = ":Given: g\n:When: w\n:Then: t"
_UID = "Suite:T"  # raw uid in gtest.lobster tag
_LABEL = "//pkg:cov"  # Bazel label set in all tests
_PACKAGE = "//pkg"  # package extracted from _LABEL


def _setup(tmp_path: Path, req_ids, uid=_UID, spec=_SPEC, committed=("g", "w", "t")):
    req_lobster = tmp_path / "reqs.lobster"
    write_req_lobster(req_lobster, req_ids)
    lm = tmp_path / "lobster_manifest.txt"
    lm.write_text(str(req_lobster) + "\n")

    gtest = tmp_path / "gtest.lobster"
    write_gtest_lobster(gtest, uid, req_ids[0] if req_ids else "X", spec=spec)

    # uid in lock must include package prefix (as scan_gtest_lobster will produce)
    scoped_uid = f"{_PACKAGE}/{uid}"
    given, when, then = committed
    lock_content = serialize(
        LockFile(
            schema_version=3,
            requirements=[
                RequirementEntry(
                    id=req_ids[0] if req_ids else "X",
                    version="1",
                    test_cases=[TestCase(uid=scoped_uid, given=given, when=when, then=then)],
                )
            ],
        )
    )
    lock = tmp_path / "test_case_coverage.lock.yaml"
    lock.write_text(lock_content)

    lobster_out = tmp_path / "out.lobster"
    return lm, gtest, lock, lobster_out


def test_clean_pass(tmp_path, monkeypatch):
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"])
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("TEST_CASE_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LABEL", _LABEL)
    monkeypatch.setenv("TEST_CASE_COVERAGE_PACKAGE", _PACKAGE)
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_OUTPUT", str(lobster_out))
    runner_main([])  # must not raise


def test_drift_fails(tmp_path, monkeypatch):
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"])
    # Tamper with the gtest.lobster so spec differs from committed
    write_gtest_lobster(gtest, _UID, "P.R", spec=":Given: CHANGED\n:When: w\n:Then: t")
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("TEST_CASE_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LABEL", _LABEL)
    monkeypatch.setenv("TEST_CASE_COVERAGE_PACKAGE", _PACKAGE)
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_OUTPUT", str(lobster_out))
    with pytest.raises(SystemExit) as exc_info:
        runner_main([])
    assert exc_info.value.code == 1


def test_drift_still_emits_lobster(tmp_path, monkeypatch):
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"])
    write_gtest_lobster(gtest, _UID, "P.R", spec=":Given: CHANGED\n:When: w\n:Then: t")
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("TEST_CASE_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LABEL", _LABEL)
    monkeypatch.setenv("TEST_CASE_COVERAGE_PACKAGE", _PACKAGE)
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_OUTPUT", str(lobster_out))
    try:
        runner_main([])
    except SystemExit:
        pass
    assert lobster_out.exists()
    data = json.loads(lobster_out.read_text())
    assert data["data"][0]["status"] == "fail"


def test_failure_prints_remediation(tmp_path, monkeypatch, capsys):
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"])
    write_gtest_lobster(gtest, _UID, "P.R", spec=":Given: CHANGED\n:When: w\n:Then: t")
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("TEST_CASE_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LABEL", _LABEL)
    monkeypatch.setenv("TEST_CASE_COVERAGE_PACKAGE", _PACKAGE)
    monkeypatch.delenv("TEST_CASE_COVERAGE_LOBSTER_OUTPUT", raising=False)
    try:
        runner_main([])
    except SystemExit:
        pass
    err = capsys.readouterr().err
    assert "bazel run" in err
    assert ".update" in err


def test_missing_committed_lock_file_fails(tmp_path, monkeypatch):
    """No committed lock file at all (e.g. first run) must be reported as a failure."""
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"])
    lock.unlink()
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("TEST_CASE_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LABEL", _LABEL)
    monkeypatch.setenv("TEST_CASE_COVERAGE_PACKAGE", _PACKAGE)
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_OUTPUT", str(lobster_out))
    with pytest.raises(SystemExit) as exc_info:
        runner_main([])
    assert exc_info.value.code == 1
    assert lobster_out.exists()  # artifact still written (D-6)


def test_malformed_committed_lock_file_fails_with_message(tmp_path, monkeypatch, capsys):
    """A lock file with a bad/mismatched schema_version must fail with a clear message."""
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"])
    lock.write_text("schema_version: 999\nrequirements: []\n")
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("TEST_CASE_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LABEL", _LABEL)
    monkeypatch.setenv("TEST_CASE_COVERAGE_PACKAGE", _PACKAGE)
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_OUTPUT", str(lobster_out))
    with pytest.raises(SystemExit) as exc_info:
        runner_main([])
    assert exc_info.value.code == 1
    err = capsys.readouterr().err
    assert "schema_version" in err


def test_allow_check_failures_suppresses_exit(tmp_path, monkeypatch):
    """--allow-check-failures must return success even when the lock check fails."""
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"])
    write_gtest_lobster(gtest, _UID, "P.R", spec=":Given: CHANGED\n:When: w\n:Then: t")
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("TEST_CASE_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LABEL", _LABEL)
    monkeypatch.setenv("TEST_CASE_COVERAGE_PACKAGE", _PACKAGE)
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_OUTPUT", str(lobster_out))
    runner_main(["--allow-check-failures"])  # must not raise


def _set_env(monkeypatch, lm, gtest, lock, lobster_out):
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("TEST_CASE_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LABEL", _LABEL)
    monkeypatch.setenv("TEST_CASE_COVERAGE_PACKAGE", _PACKAGE)
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_OUTPUT", str(lobster_out))


def test_spec_without_when_passes(tmp_path, monkeypatch):
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"], spec=":Given: g\n:Then: t", committed=("g", "", "t"))
    _set_env(monkeypatch, lm, gtest, lock, lobster_out)
    runner_main([])  # must not raise


def test_missing_then_fails_without_drift(tmp_path, monkeypatch, capsys):
    """The lock matches the tests, so only the missing Then can fail the run."""
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"], spec=":Given: g\n:When: w", committed=("g", "w", ""))
    _set_env(monkeypatch, lm, gtest, lock, lobster_out)
    with pytest.raises(SystemExit) as exc_info:
        runner_main([])
    assert exc_info.value.code == 1
    assert "(then)" in capsys.readouterr().err
    assert lobster_out.exists()  # artifact still written (D-6)


def test_missing_given_fails_without_drift(tmp_path, monkeypatch, capsys):
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"], spec=":When: w\n:Then: t", committed=("", "w", "t"))
    _set_env(monkeypatch, lm, gtest, lock, lobster_out)
    with pytest.raises(SystemExit) as exc_info:
        runner_main([])
    assert exc_info.value.code == 1
    assert "(given)" in capsys.readouterr().err


def test_missing_then_allowed_with_allow_check_failures(tmp_path, monkeypatch):
    lm, gtest, lock, lobster_out = _setup(tmp_path, ["P.R"], spec=":Given: g\n:When: w", committed=("g", "w", ""))
    _set_env(monkeypatch, lm, gtest, lock, lobster_out)
    runner_main(["--allow-check-failures"])  # must not raise
    assert lobster_out.exists()


# ---------------------------------------------------------------------------
# Formal test specification (test-spec)
# ---------------------------------------------------------------------------


def _setup_spec(tmp_path: Path, monkeypatch, committed_goal="goal", spec_goal="goal"):
    req_lobster = tmp_path / "reqs.lobster"
    write_req_lobster(req_lobster, ["P.R"])
    lm = tmp_path / "lobster_manifest.txt"
    lm.write_text(str(req_lobster) + "\n")
    gtest = tmp_path / "gtest.lobster"
    write_gtest_lobster(gtest, _UID, "P.R", spec=":Test-spec: //pkg:spec")

    committed_spec = TestSpec(ref="//pkg:spec", goal=committed_goal, steps=(TestStep(action="a", expected="e"),))
    lock = tmp_path / "test_case_coverage.lock.yaml"
    lock.write_text(
        serialize(
            LockFile(
                schema_version=3,
                requirements=[
                    RequirementEntry(
                        id="P.R",
                        version="1",
                        test_cases=[TestCase(uid=f"{_PACKAGE}/{_UID}", spec=committed_spec)],
                    )
                ],
            )
        )
    )
    lobster_out = tmp_path / "out.lobster"
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("TEST_CASE_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("TEST_CASE_COVERAGE_LABEL", _LABEL)
    monkeypatch.setenv("TEST_CASE_COVERAGE_PACKAGE", _PACKAGE)
    monkeypatch.setenv("TEST_CASE_COVERAGE_LOBSTER_OUTPUT", str(lobster_out))
    monkeypatch.setenv("TEST_CASE_COVERAGE_SPEC_MANIFEST", str(write_test_spec_manifest(tmp_path, goal=spec_goal)))
    return lobster_out


def test_test_spec_clean_pass(tmp_path, monkeypatch):
    lobster_out = _setup_spec(tmp_path, monkeypatch)
    runner_main([])  # must not raise
    item = json.loads(lobster_out.read_text())["data"][0]
    assert item["status"] == "ok"
    assert ":Test specification: ``//pkg:spec``" in item["text"]


def test_test_spec_content_drift_fails(tmp_path, monkeypatch, capsys):
    _setup_spec(tmp_path, monkeypatch, spec_goal="changed goal")
    with pytest.raises(SystemExit):
        runner_main([])
    assert "test-spec goal changed" in capsys.readouterr().err


if __name__ == "__main__":
    import sys

    sys.exit(pytest.main([__file__, "-v"]))
