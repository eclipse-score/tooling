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
"""YAML lock-file serialization/deserialization and lock computation."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml

# Version of the lock-file schema produced by this module.
SCHEMA_VERSION: int = 3


@dataclass
class TestCase:
    uid: str  # "Suite:TestName" — matches gtest.lobster tag (without namespace)
    given: str = ""  # :Given: text from RecordProperty annotation
    when: str = ""  # :When: text
    then: str = ""  # :Then: text


@dataclass
class RequirementMeta:
    """Metadata for a single requirement read from a lobster-req-trace file."""

    id: str  # noqa: A003  e.g. "SampleComponent.REQ_COMP_001"
    version: str = ""  # from TRLC tag @version suffix, e.g. "1"
    description: str = ""  # from TRLC text field; stored for display only


@dataclass
class RequirementEntry:
    id: str  # noqa: A003
    version: str = ""  # checked on drift — version bump requires re-approval
    description: str = ""  # display only; not compared in lock checks
    test_cases: list[TestCase] = field(default_factory=list)


@dataclass
class LockFile:
    schema_version: int
    requirements: list[RequirementEntry] = field(default_factory=list)

    def to_yaml(self) -> str:
        """Serialize to YAML string."""
        return serialize(self)

    @staticmethod
    def from_yaml(data: dict[str, Any]) -> "LockFile":
        """Deserialize from YAML dict."""
        return _from_dict(data)


# ---------------------------------------------------------------------------
# Serialization
# ---------------------------------------------------------------------------


def _tc_to_dict(tc: TestCase) -> dict[str, Any]:
    return {
        "uid": tc.uid,
        "given": tc.given,
        "when": tc.when,
        "then": tc.then,
    }


def _req_to_dict(req: RequirementEntry) -> dict[str, Any]:
    d: dict[str, Any] = {"id": req.id}
    if req.version:
        d["version"] = req.version
    if req.description:
        d["description"] = req.description
    d["test_cases"] = [_tc_to_dict(tc) for tc in req.test_cases]
    return d


def serialize(lock: LockFile) -> str:
    """Serialize a LockFile to a YAML string."""
    data: dict[str, Any] = {
        "schema_version": lock.schema_version,
        "requirements": [_req_to_dict(r) for r in lock.requirements],
    }
    return yaml.dump(
        data, default_flow_style=False, allow_unicode=True, sort_keys=False
    )


# ---------------------------------------------------------------------------
# Deserialization
# ---------------------------------------------------------------------------


def _tc_from_dict(d: dict[str, Any]) -> TestCase:
    return TestCase(
        uid=str(d.get("uid", "")),
        given=str(d.get("given", "")),
        when=str(d.get("when", "")),
        then=str(d.get("then", "")),
    )


def _req_from_dict(d: dict[str, Any]) -> RequirementEntry:
    return RequirementEntry(
        id=str(d["id"]),
        version=str(d.get("version", "")),
        description=str(d.get("description", "")),
        test_cases=[_tc_from_dict(tc) for tc in d.get("test_cases", [])],
    )


def _from_dict(data: dict[str, Any]) -> LockFile:
    """Deserialize from YAML dict."""
    if not isinstance(data, dict):
        raise ValueError("Lock file must be a YAML mapping")

    schema_version = int(data.get("schema_version", 0))
    requirements = [_req_from_dict(r) for r in data.get("requirements", [])]
    return LockFile(schema_version=schema_version, requirements=requirements)


def deserialize(yaml_str: str) -> LockFile:
    """Deserialize a YAML string into a LockFile."""
    try:
        data = yaml.safe_load(yaml_str)
    except yaml.YAMLError as exc:
        raise ValueError(f"Invalid lock file YAML: {exc}") from exc

    return _from_dict(data)


def load_lock_file(path: Path) -> LockFile:
    """Read and deserialize a lock file from disk."""
    try:
        return deserialize(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ValueError(f"Cannot read lock file {path}: {exc}") from exc


# ---------------------------------------------------------------------------
# Computation from scan results
# ---------------------------------------------------------------------------


def compute_lock(
    req_metadata: list["RequirementMeta"],
    test_cases_by_req: dict[str, list],  # list[TestRecord] from read_gtest_lobster
) -> LockFile:
    """Build a LockFile from scanned test records, sorted deterministically.

    Each RequirementEntry stores version (checked on drift) and description
    (display only).  Each TestCase stores uid and GWT fields.
    """
    requirements: list[RequirementEntry] = []
    for meta in req_metadata:
        records = test_cases_by_req.get(meta.id, [])
        test_cases: list[TestCase] = [
            TestCase(uid=rec.uid, given=rec.given, when=rec.when, then=rec.then)
            for rec in records
        ]
        # Sort lexicographically by uid for determinism
        test_cases.sort(key=lambda tc: tc.uid)
        requirements.append(
            RequirementEntry(
                id=meta.id,
                version=meta.version,
                description=meta.description,
                test_cases=test_cases,
            )
        )

    # Sort requirements by ID for determinism
    requirements.sort(key=lambda r: r.id)
    return LockFile(schema_version=SCHEMA_VERSION, requirements=requirements)
