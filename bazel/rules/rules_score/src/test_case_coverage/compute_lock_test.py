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
from test_case_coverage.compute_lock import (
    SCHEMA_VERSION,
    LockFile,
    RequirementEntry,
    RequirementMeta,
    TestCase,
    compute_lock,
    deserialize,
    serialize,
)
from test_case_coverage.read_gtest_lobster import TestRecord


def _meta(req_id: str, version: str = "1", description: str = "") -> RequirementMeta:
    return RequirementMeta(id=req_id, version=version, description=description)


def test_testcase_uid_and_spec():
    """Test that TestCase stores uid and given/when/then correctly."""
    tc = TestCase(
        uid="Suite:TestName",
        given="g",
        when="w",
        then="t",
    )
    assert tc.uid == "Suite:TestName"
    assert tc.given == "g"
    assert tc.when == "w"
    assert tc.then == "t"


def test_serialize_roundtrip():
    lock = LockFile(
        schema_version=3,
        requirements=[
            RequirementEntry(
                id="Pkg.Req",
                version="2",
                description="Some description",
                test_cases=[
                    TestCase(
                        uid="Suite:Case",
                        given="g",
                        when="w",
                        then="t",
                    )
                ],
            )
        ],
    )
    yaml_str = serialize(lock)
    restored = deserialize(yaml_str)
    assert restored.schema_version == 3
    assert len(restored.requirements) == 1
    req = restored.requirements[0]
    assert req.id == "Pkg.Req"
    assert req.version == "2"
    assert req.description == "Some description"
    assert len(req.test_cases) == 1
    tc = req.test_cases[0]
    assert tc.uid == "Suite:Case"
    assert tc.given == "g"
    assert tc.when == "w"
    assert tc.then == "t"


def test_deserialize_missing_spec_defaults_to_empty():
    yaml_str = "schema_version: 3\nrequirements:\n  - id: P.R\n    test_cases:\n      - uid: Suite:Test\n        given: ''\n        when: ''\n        then: ''\n"
    lock = deserialize(yaml_str)
    tc = lock.requirements[0].test_cases[0]
    assert tc.given == ""
    assert tc.when == ""
    assert tc.then == ""


def test_deserialize_invalid_yaml_raises():
    import pytest

    with pytest.raises(ValueError, match="Invalid lock file YAML"):
        deserialize("{bad: [yaml")


def test_deserialize_wrong_schema_version_raises():
    import pytest

    yaml_str = "schema_version: 999\nrequirements: []\n"
    with pytest.raises(ValueError, match="schema_version"):
        deserialize(yaml_str)


def test_deserialize_matching_schema_version_ok():
    yaml_str = f"schema_version: {SCHEMA_VERSION}\nrequirements: []\n"
    lock = deserialize(yaml_str)
    assert lock.schema_version == SCHEMA_VERSION
    assert lock.requirements == []


def test_deserialize_requirement_missing_id_raises():
    import pytest

    yaml_str = f"schema_version: {SCHEMA_VERSION}\nrequirements:\n  - version: '1'\n    test_cases: []\n"
    with pytest.raises(ValueError, match="missing required 'id'"):
        deserialize(yaml_str)


def test_compute_lock_sorts_test_cases_by_uid():
    records = {
        "P.R": [
            TestRecord(uid="Suite:B", lobster_traces=["P.R"]),
            TestRecord(uid="Suite:A", lobster_traces=["P.R"]),
        ]
    }
    lock = compute_lock([_meta("P.R")], records)
    uids = [tc.uid for tc in lock.requirements[0].test_cases]
    assert uids == ["Suite:A", "Suite:B"]


def test_compute_lock_sorts_requirements():
    lock = compute_lock([_meta("P.B"), _meta("P.A")], {"P.A": [], "P.B": []})
    ids = [r.id for r in lock.requirements]
    assert ids == ["P.A", "P.B"]


def test_compute_lock_missing_req_yields_empty_test_cases():
    lock = compute_lock([_meta("P.R")], {})
    assert lock.requirements[0].test_cases == []


def test_compute_lock_builds_gwt_from_record():
    records = {"P.R": [TestRecord(uid="Suite:T", lobster_traces=["P.R"], given="g", when="w", then="t")]}
    lock = compute_lock([_meta("P.R")], records)
    tc = lock.requirements[0].test_cases[0]
    assert tc.given == "g"
    assert tc.when == "w"
    assert tc.then == "t"


def test_compute_lock_stores_version_and_description():
    lock = compute_lock([_meta("P.R", version="3", description="desc")], {})
    req = lock.requirements[0]
    assert req.version == "3"
    assert req.description == "desc"


def test_compute_lock_threads_gtest_tag():
    """gtest_tag from the scanned record must reach the TestCase, so
    lobster_generator.py can build a native cross-reference to the
    underlying gtest lobster item."""
    records = {
        "P.R": [
            TestRecord(
                uid="//pkg/Suite:T",
                lobster_traces=["P.R"],
                gtest_tag="Suite:T",
            )
        ]
    }
    lock = compute_lock([_meta("P.R")], records)
    tc = lock.requirements[0].test_cases[0]
    assert tc.gtest_tag == "Suite:T"


def test_gtest_tag_excluded_from_serialized_yaml():
    """This field is a raw gtest tag, sandbox/build-derived, and must never
    leak into the committed lock file (that would cause spurious lock drift).
    """
    lock = LockFile(
        schema_version=3,
        requirements=[
            RequirementEntry(
                id="P.R",
                test_cases=[TestCase(uid="Suite:T", gtest_tag="Suite:T")],
            )
        ],
    )
    yaml_str = serialize(lock)
    assert "gtest_tag" not in yaml_str
    restored = deserialize(yaml_str)
    tc = restored.requirements[0].test_cases[0]
    assert tc.gtest_tag == ""


if __name__ == "__main__":
    import sys
    import pytest

    sys.exit(pytest.main([__file__, "-v"]))
