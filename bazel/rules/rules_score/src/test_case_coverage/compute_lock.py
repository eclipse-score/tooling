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

from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import TYPE_CHECKING, Any

import yaml

if TYPE_CHECKING:
    from test_case_coverage.read_gtest_lobster import TestRecord

# Version of the lock-file schema produced by this module.
SCHEMA_VERSION: int = 3


@dataclass(frozen=True)
class TestStep:
    """One step of a formal RST test specification."""

    __test__ = False  # not a pytest test class

    precondition: str = ""  # optional
    action: str = ""
    expected: str = ""


@dataclass(frozen=True)
class TestSpec:
    """Formal test specification parsed from a ``test_specification`` RST file.

    Replaces Given/When/Then for every test case that references it through
    ``RecordProperty("test-spec", "<label>")``.
    """

    __test__ = False  # not a pytest test class

    ref: str  # label of the test_specification, as referenced by the test case
    goal: str = ""
    focus: str = ""  # optional
    steps: tuple[TestStep, ...] = ()


@dataclass
class TestCase:
    uid: str  # "Suite:TestName" — matches gtest.lobster tag (without namespace)
    given: str = ""  # :Given: text from RecordProperty or the test name
    when: str = ""  # :When: text; may be empty
    then: str = ""  # :Then: text
    # Formal RST test specification; when set, given/when/then are empty and
    # the lock entry carries a ``spec`` mapping instead of the three GWT keys.
    spec: TestSpec | None = None
    # Raw ``test-spec`` reference as written in the test (computed side only,
    # never serialized). Set while ``spec`` is None means it did not resolve.
    spec_ref: str = ""
    # Populated only on the freshly *computed* side (never read from / written
    # to the committed lock YAML): it's the raw, unprefixed gtest tag, used by
    # lobster_generator.py to add a native cross-reference to the underlying
    # gtest lobster item for each test case.
    gtest_tag: str = ""  # raw, unprefixed "Suite:TestName" tag


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


def normalize_label(label: str) -> str:
    """Normalize a Bazel label of the main repository to ``//pkg:name``.

    Accepts ``@@//pkg:name``, ``@//pkg:name`` and the shorthand ``//pkg/name``
    (meaning ``//pkg/name:name``), so a test may reference a
    ``test_specification`` in any of these spellings.
    """
    label = label.strip()
    for prefix in ("@@//", "@//"):
        if label.startswith(prefix):
            label = label[len(prefix) - 2 :]
            break
    if label.startswith("//") and ":" not in label:
        label = f"{label}:{label.rsplit('/', 1)[-1]}"
    return label


# ---------------------------------------------------------------------------
# Serialization
# ---------------------------------------------------------------------------


def spec_to_dict(spec: TestSpec) -> dict[str, Any]:
    """Serialize a TestSpec to a plain mapping (lock file and JSON exchange format)."""
    return {
        "ref": spec.ref,
        "goal": spec.goal,
        "focus": spec.focus,
        "steps": [{"precondition": s.precondition, "action": s.action, "expected": s.expected} for s in spec.steps],
    }


def spec_from_dict(d: Any) -> TestSpec:
    """Deserialize a TestSpec from a mapping produced by :func:`spec_to_dict`."""
    if not isinstance(d, dict) or not d.get("ref"):
        raise ValueError(f"Test specification must be a mapping with a non-empty 'ref': {d!r}")
    steps = d.get("steps") or []
    if not isinstance(steps, list):
        raise ValueError(f"Test specification 'steps' must be a list: {d!r}")
    return TestSpec(
        ref=str(d["ref"]),
        goal=str(d.get("goal", "")),
        focus=str(d.get("focus", "")),
        steps=tuple(
            TestStep(
                precondition=str(s.get("precondition", "")),
                action=str(s.get("action", "")),
                expected=str(s.get("expected", "")),
            )
            for s in steps
        ),
    )


def _tc_to_dict(tc: TestCase) -> dict[str, Any]:
    if tc.spec is not None:
        return {"uid": tc.uid, "spec": spec_to_dict(tc.spec)}
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


class _LockDumper(yaml.SafeDumper):
    """Dumps multi-line strings (e.g. test specification text) as readable literal blocks."""


def _represent_str(dumper: yaml.SafeDumper, value: str) -> yaml.Node:
    style = "|" if "\n" in value else None
    return dumper.represent_scalar("tag:yaml.org,2002:str", value, style=style)


_LockDumper.add_representer(str, _represent_str)


def serialize(lock: LockFile) -> str:
    """Serialize a LockFile to a YAML string."""
    data: dict[str, Any] = {
        "schema_version": lock.schema_version,
        "requirements": [_req_to_dict(r) for r in lock.requirements],
    }
    return yaml.dump(data, Dumper=_LockDumper, default_flow_style=False, allow_unicode=True, sort_keys=False)


# ---------------------------------------------------------------------------
# Deserialization
# ---------------------------------------------------------------------------


_GWT_FIELDS = ("given", "when", "then")


def _tc_from_dict(d: dict[str, Any]) -> TestCase:
    if "spec" in d:
        mixed = [k for k in _GWT_FIELDS if k in d]
        if mixed:
            raise ValueError(
                f"Lock file test case {d.get('uid')!r} has both 'spec' and {', '.join(mixed)}; "
                "a test case is specified either by 'spec' or by given/when/then."
            )
        return TestCase(uid=str(d.get("uid", "")), spec=spec_from_dict(d["spec"]))
    return TestCase(
        uid=str(d.get("uid", "")),
        given=str(d.get("given", "")),
        when=str(d.get("when", "")),
        then=str(d.get("then", "")),
    )


def _req_from_dict(d: dict[str, Any]) -> RequirementEntry:
    if "id" not in d:
        raise ValueError(f"Lock file requirement entry missing required 'id' field: {d!r}")
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
    if schema_version != SCHEMA_VERSION:
        raise ValueError(
            f"Lock file schema_version is {schema_version}, but this tool expects "
            f"{SCHEMA_VERSION}. This is a lock *format* mismatch, not a coverage "
            "drift -- run the update runner to regenerate it in the current "
            "format, then review and commit the diff like any other lock file "
            "change."
        )
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


def _record_to_test_case(rec: "TestRecord", specs: dict[str, TestSpec | None]) -> TestCase:
    if not rec.spec_ref:
        return TestCase(uid=rec.uid, given=rec.given, when=rec.when, then=rec.then, gtest_tag=rec.gtest_tag)
    # A spec replaces given/when/then. GWT that lobster-gtest derived from the
    # test name (or set as properties) is dropped instead of being mixed in.
    reference = normalize_label(rec.spec_ref)
    spec = specs.get(reference)
    return TestCase(
        uid=rec.uid,
        # Record the reference as written in the test: it is what the reviewer
        # sees in the source and it does not depend on canonical repo names.
        spec=replace(spec, ref=reference) if spec is not None else None,
        spec_ref=rec.spec_ref,
        gtest_tag=rec.gtest_tag,
    )


def compute_lock(
    req_metadata: list[RequirementMeta],
    test_cases_by_req: dict[str, list["TestRecord"]],
    specs: dict[str, TestSpec | None] | None = None,
) -> LockFile:
    """Build a LockFile from scanned test records, sorted deterministically.

    Each RequirementEntry stores version (checked on drift) and description
    (display only).  Each TestCase stores uid and either the GWT fields or,
    for a record with a ``test-spec`` reference, the resolved TestSpec from
    *specs* (keyed by :func:`normalize_label`).
    """
    requirements: list[RequirementEntry] = []
    for meta in req_metadata:
        records = test_cases_by_req.get(meta.id, [])
        test_cases: list[TestCase] = [_record_to_test_case(rec, specs or {}) for rec in records]
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
