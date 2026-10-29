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
"""Compare a committed lock file against a freshly computed one."""

from __future__ import annotations

from test_case_coverage.compute_lock import LockFile, TestCase, TestSpec


def _req_map_by_uid(lock: LockFile) -> dict[str, dict[str, TestCase]]:
    """Index a LockFile as {req_id: {uid: TestCase}}."""
    return {req.id: {tc.uid: tc for tc in req.test_cases} for req in lock.requirements}


def _req_version_map(lock: LockFile) -> dict[str, str]:
    """Index a LockFile as {req_id: version}."""
    return {req.id: req.version for req in lock.requirements}


def compare_lock_files(
    committed: LockFile,
    computed: LockFile,
) -> tuple[bool, list[str]]:
    """Return (ok, diff_lines) comparing committed vs computed lock.

    Checks (in order per requirement):
    1. Requirement added / removed.
    2. Version changed (triggers re-approval).
    3. Test cases added / removed / GWT or test specification changed.

    Description is intentionally excluded from comparison (display only).
    diff_lines is empty when ok is True.
    """
    diff: list[str] = []

    committed_map = _req_map_by_uid(committed)
    computed_map = _req_map_by_uid(computed)
    committed_ver = _req_version_map(committed)
    computed_ver = _req_version_map(computed)

    all_req_ids = sorted(set(committed_map) | set(computed_map))

    for req_id in all_req_ids:
        if req_id not in committed_map:
            diff.append(f"  + requirement added: {req_id}")
            continue
        if req_id not in computed_map:
            diff.append(f"  - requirement removed: {req_id}")
            continue

        # Version check — a version bump needs explicit re-approval.
        comm_ver = committed_ver.get(req_id, "")
        comp_ver = computed_ver.get(req_id, "")
        if comm_ver != comp_ver:
            diff.append(f"  ~ [{req_id}] version changed: {comm_ver!r} → {comp_ver!r}")

        comm_tests = committed_map[req_id]
        comp_tests = computed_map[req_id]
        all_uids = sorted(set(comm_tests) | set(comp_tests))

        for uid in all_uids:
            if uid not in comm_tests:
                diff.append(f"  + [{req_id}] test added: {uid}")
            elif uid not in comp_tests:
                diff.append(f"  - [{req_id}] test removed: {uid}")
            else:
                comm_tc = comm_tests[uid]
                comp_tc = comp_tests[uid]
                diff.extend(_spec_diff(req_id, uid, comm_tc.spec, comp_tc.spec))
                for field_name, comm_val, comp_val in (
                    ("given", comm_tc.given, comp_tc.given),
                    ("when", comm_tc.when, comp_tc.when),
                    ("then", comm_tc.then, comp_tc.then),
                ):
                    if comm_val != comp_val:
                        diff.append(
                            f"  ~ [{req_id}] {field_name} changed: {uid}\n"
                            f"      lockfile: {comm_val!r}\n"
                            f"      testcase: {comp_val!r}"
                        )

    return len(diff) == 0, diff


def _spec_diff(req_id: str, uid: str, committed: TestSpec | None, computed: TestSpec | None) -> list[str]:
    """Describe how the test specification of one test case changed."""
    if committed == computed:
        return []
    if committed is None or computed is None:
        before = "given/when/then" if committed is None else f"test-spec {committed.ref}"
        after = "given/when/then" if computed is None else f"test-spec {computed.ref}"
        return [f"  ~ [{req_id}] specification changed from {before} to {after}: {uid}"]
    lines = []
    for field_name, comm_val, comp_val in (
        ("ref", committed.ref, computed.ref),
        ("goal", committed.goal, computed.goal),
        ("focus", committed.focus, computed.focus),
        ("steps", committed.steps, computed.steps),
    ):
        if comm_val != comp_val:
            lines.append(
                f"  ~ [{req_id}] test-spec {field_name} changed: {uid}\n"
                f"      lockfile: {comm_val!r}\n"
                f"      testcase: {comp_val!r}"
            )
    return lines


def validate_specs(computed: LockFile) -> tuple[bool, list[str]]:
    """Fail if any test case in *computed* is not specified.

    A test case is specified either by a resolved ``test-spec`` reference, or
    by a ``given`` and a ``then`` part. ``when`` is optional, because a test
    name derived by lobster-gtest may omit it. Coverage without a specification
    cannot be meaningfully reviewed or approved.

    Returns (ok, issue_lines).
    """
    issues: list[str] = []
    for req in computed.requirements:
        for tc in req.test_cases:
            if tc.spec is not None:
                continue
            if tc.spec_ref:
                issues.append(
                    f"  ! [{req.id}] test-spec {tc.spec_ref!r} does not resolve: {tc.uid}\n"
                    f"    Add the test_specification to the unit's test_specs and reference it in the test "
                    f"with the same label as written there (e.g. //pkg:my_spec)."
                )
                continue
            missing = [name for name, val in (("given", tc.given), ("then", tc.then)) if not val.strip()]
            if missing:
                issues.append(
                    f"  ! [{req.id}] test missing GWT fields ({', '.join(missing)}): {tc.uid}\n"
                    f"    Set RecordProperty 'given' and 'then' ('when' is optional). Alternatively, name the test "
                    f"Given<X>[_When<Y>]_(Expect|Then)<Z> if lobster provides --derive-gwt-from-test-name, "
                    f"or reference a test_specification with RecordProperty 'test-spec'."
                )
    return len(issues) == 0, issues
