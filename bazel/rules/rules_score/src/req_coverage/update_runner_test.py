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
from pathlib import Path

import pytest

from req_coverage.compute_lock import deserialize
from req_coverage.update_runner import main as update_main
from req_coverage.conftest import write_req_lobster, write_gtest_lobster

_SPEC = ":Given: g\n:When: w\n:Then: t"
_UID = "Suite:T"


def _setup(tmp_path: Path, req_ids, uid=_UID, trace=None):
    req_lobster = tmp_path / "reqs.lobster"
    write_req_lobster(req_lobster, req_ids)

    lm = tmp_path / "lobster_manifest.txt"
    lm.write_text(str(req_lobster) + "\n", encoding="utf-8")

    gtest = tmp_path / "gtest.lobster"
    traced_req = trace or (req_ids[0] if req_ids else "X")
    write_gtest_lobster(gtest, uid, traced_req)

    lock = tmp_path / "coverage.lock.yaml"
    return lm, gtest, lock


def test_update_writes_lock_file(tmp_path, monkeypatch):
    lm, gtest, lock = _setup(tmp_path, ["P.R"])
    monkeypatch.setenv("REQ_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("REQ_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("REQ_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("REQ_COVERAGE_LABEL", "//pkg:cov")
    monkeypatch.setenv("REQ_COVERAGE_PACKAGE", "//pkg")
    update_main()
    assert lock.exists()
    parsed = deserialize(lock.read_text())
    assert parsed.requirements[0].id == "P.R"
    assert parsed.requirements[0].version == "1"
    assert parsed.requirements[0].test_cases[0].uid == f"//pkg/{_UID}"


def test_update_warns_on_zero_linked_tests(tmp_path, monkeypatch, capsys):
    lm, gtest, lock = _setup(tmp_path, ["P.R", "P.NoTest"], trace="P.R")
    monkeypatch.setenv("REQ_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("REQ_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("REQ_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("REQ_COVERAGE_LABEL", "//pkg:cov")
    monkeypatch.setenv("REQ_COVERAGE_PACKAGE", "//pkg")
    update_main()  # must not raise
    captured = capsys.readouterr()
    assert "P.NoTest" in captured.err
    assert "WARNING" in captured.err


def test_update_exits_0_on_zero_linked_tests(tmp_path, monkeypatch):
    lm, gtest, lock = _setup(tmp_path, ["P.NoTest"], trace="NOMATCH")
    monkeypatch.setenv("REQ_COVERAGE_LOBSTER_MANIFEST", str(lm))
    monkeypatch.setenv("REQ_COVERAGE_GTEST_LOBSTER", str(gtest))
    monkeypatch.setenv("REQ_COVERAGE_LOCK_FILE", str(lock))
    monkeypatch.setenv("REQ_COVERAGE_LABEL", "//pkg:cov")
    monkeypatch.setenv("REQ_COVERAGE_PACKAGE", "//pkg")
    update_main()  # should not raise SystemExit


def test_update_missing_env_exits_1(monkeypatch):
    for var in (
        "REQ_COVERAGE_LOBSTER_MANIFEST",
        "REQ_COVERAGE_GTEST_LOBSTER",
        "REQ_COVERAGE_LOCK_FILE",
    ):
        monkeypatch.delenv(var, raising=False)
    with pytest.raises(SystemExit) as exc_info:
        update_main()
    assert exc_info.value.code == 1


if __name__ == "__main__":
    import sys

    sys.exit(pytest.main([__file__, "-v"]))
