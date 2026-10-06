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
"""Read test records from lobster-gtest JSON artifacts.

Provides all lobster-file reading utilities used by both runners:

* :func:`read_gtest_lobster` — parse a single gtest.lobster file
* :func:`scan_gtest_lobster` — group test records by requirement ID
* :func:`read_req_metadata_from_lobster_files` — extract CompReq metadata from a manifest
* :func:`resolve_path` — resolve Bazel-relative path in action/runfiles contexts
"""

from __future__ import annotations

import logging
import os
import re
from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import NamedTuple

from lobster.common.errors import LOBSTER_Error, Message_Handler
from lobster.common.io import lobster_read
from lobster.common.items import Activity, Requirement

from test_case_coverage.compute_lock import RequirementMeta

logger = logging.getLogger(__name__)


@dataclass
class TestRecord:
    """Metadata extracted from a single gtest.lobster item."""

    uid: str  # "Suite:TestName" (gtest tag without "gtest " prefix)
    lobster_traces: list[str] = field(default_factory=list)  # requirement IDs
    given: str = ""  # :Given: text from RecordProperty annotation, or derived from the test name
    when: str = ""  # :When: text, or derived from the test name
    then: str = ""  # :Then: text, or derived from the test name
    gtest_tag: str = ""  # raw, unprefixed "Suite:TestName" tag — survives the
    # package-prefixing done to `uid` in scan_gtest_lobster, so it always
    # matches the Tracing_Tag("gtest", ...) of the underlying gtest item.


class _GwtComponent(NamedTuple):
    """One Given-When-Then component and the ways it can be spelled."""

    name: str  # component name, matching the TestRecord field
    annotation_key: str  # line prefix in the lobster text, e.g. ":Given:"
    name_prefixes: tuple[str, ...]  # accepted prefixes in a gtest test case name


# Single source of truth for the three components. lobster-gtest capitalises
# RecordProperty keys, hence the ":Given:" style annotation keys; gtest test case names
# follow the convention "(<MemberFunction>_)?Given<Context>(_When<Condition>)_Expect<Result>".
# The result part is commonly spelled either "Expect<Result>" or "Then<Result>", so both
# prefixes are accepted; the first matching prefix of a component wins.
_GWT_COMPONENTS: tuple[_GwtComponent, ...] = (
    _GwtComponent("given", ":Given:", ("Given",)),
    _GwtComponent("when", ":When:", ("When",)),
    _GwtComponent("then", ":Then:", ("Expect", "Then")),
)

# A test case name is split at "_", which separates the parts of the name itself, and at
# "/", which separates the name from the name of a test parameter.
_TEST_CASE_NAME_SEPARATORS = re.compile(r"[_/]")


def _empty_components() -> dict[str, str]:
    """Return a mapping of every component name to an empty value."""
    return {component.name: "" for component in _GWT_COMPONENTS}


def _as_triple(values: dict[str, str]) -> tuple[str, str, str]:
    """Return the component values in ``(given, when, then)`` order."""
    given, when, then = (values[component.name] for component in _GWT_COMPONENTS)
    return given, when, then


def _match_name_prefix(part: str) -> tuple[str, str] | None:
    """Match one part of a test case name against the known component prefixes.

    Returns ``(component_name, remainder)`` for the first matching prefix, or ``None``
    if the part belongs to no component (e.g. a leading ``<MemberFunction>`` part or an
    uninformative test parameter name).
    """
    for component in _GWT_COMPONENTS:
        for prefix in component.name_prefixes:
            if part.startswith(prefix):
                return component.name, part[len(prefix) :]
    return None


def _camel_case_to_sentence(camel_case: str) -> str:
    """Convert a CamelCase identifier into a lower-case, space separated sentence.

    ``"KernelLargerThanSignal"`` becomes ``"kernel larger than signal"``.  A word
    boundary is detected at every upper-case character that either follows a
    lower-case character or is followed by one, which keeps acronyms together:
    ``"ValidPMRAllocator"`` becomes ``"valid pmr allocator"``.
    """
    characters: list[str] = []
    for index, character in enumerate(camel_case):
        follows_lower = index > 0 and camel_case[index - 1].islower()
        precedes_lower = (index + 1) < len(camel_case) and camel_case[index + 1].islower()
        if character.isupper() and index > 0 and (follows_lower or precedes_lower):
            characters.append(" ")
        characters.append(character.lower())
    return "".join(characters)


def _parse_test_case_name(test_case_name: str) -> tuple[str, str, str]:
    """Derive ``(given, when, then)`` from a gtest test case name.

    The name is expected to follow the naming convention
    ``(<MemberFunction>_)?Given<Context>(_When<Condition>)_Expect<Result>``, where
    each part is written in CamelCase.

    For a value-parameterized test, gtest appends the name of the test parameter,
    separated by ``/``.  The parameter name is treated as a further part, which
    supports both an uninformative parameter name
    (``"GivenKernelEmpty_ExpectNoResult/0"``) and the convention being carried by the
    parameter name itself (``"CheckValidMass/GivenMassBelowZero_ExpectContractViolated"``).

    The result part may be spelled either ``Expect<Result>`` or ``Then<Result>``.

    Any part that does not start with a known prefix is ignored, which covers a leading
    ``<MemberFunction>`` part as well as an uninformative parameter name.  Parts that
    are absent from the name yield an empty string.
    """
    values = _empty_components()
    for part in _TEST_CASE_NAME_SEPARATORS.split(test_case_name):
        match = _match_name_prefix(part)
        if match is not None:
            component_name, remainder = match
            values[component_name] = _camel_case_to_sentence(remainder)
    return _as_triple(values)


def _gtest_case_name_from_uid(uid: str) -> str:
    """Extract the gtest test case name from a ``"Suite:TestName"`` lobster uid."""
    return uid.rsplit(":", 1)[-1]


def _parse_gwt(text: str, test_case_name: str = "") -> tuple[str, str, str]:
    """Parse a ``:Given:/:When:/:Then:`` text field into its three components.

    lobster-gtest capitalises RecordProperty keys, so the expected format is::

        :Given: value
        :When: value
        :Then: value

    Handles a few real-world variations robustly:

    * A missing trailing space after the ``:`` (e.g. ``:Given:value`` or an
      empty value ``:Given:``) is accepted.
    * A value that wraps onto following lines is joined (space-separated)
      until the next recognised key or the end of the text.
    * If a key appears more than once, the last occurrence wins.

    If *test_case_name* is given, any component that no RecordProperty annotation
    provides is derived from the test case name instead (see
    :func:`_parse_test_case_name`).  Components that neither the annotations nor the
    name provide stay empty, so a test that follows neither convention behaves
    exactly as before.
    """
    values = _empty_components()
    current: str | None = None
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        matched_key = None
        for component in _GWT_COMPONENTS:
            if line.startswith(component.annotation_key):
                values[component.name] = line[len(component.annotation_key) :].strip()
                matched_key = component.name
                break
        if matched_key is not None:
            current = matched_key
        elif current is not None:
            values[current] = f"{values[current]} {line}".strip()

    if test_case_name:
        derived = _parse_test_case_name(test_case_name)
        for component, value in zip(_GWT_COMPONENTS, derived):
            if not values[component.name]:
                values[component.name] = value

    return _as_triple(values)


def read_gtest_lobster(lobster_path: Path) -> list[TestRecord]:
    """Parse a single gtest.lobster JSON file and return test records.

    Only items with at least one ``req`` reference are returned; items that
    carry no tracing annotation are silently skipped.

    Args:
        lobster_path: Path to a ``lobster-act-trace`` JSON file produced by
                      ``lobster-gtest``.

    Raises:
        ValueError: if the file cannot be read or parsed.
    """
    mh = Message_Handler()
    items: dict = {}
    try:
        lobster_read(mh, str(lobster_path), "act", items)
    except (OSError, LOBSTER_Error) as exc:
        raise ValueError(f"Cannot parse gtest.lobster {lobster_path}: {exc}") from exc

    records: list[TestRecord] = []
    for item in items.values():
        if not isinstance(item, Activity):
            continue
        if item.tag.namespace != "gtest":
            continue

        refs = [ref.tag for ref in item.unresolved_references if ref.namespace == "req"]
        if not refs:
            continue

        gwt = _parse_gwt(item.text or "", _gtest_case_name_from_uid(item.tag.tag))
        records.append(
            TestRecord(
                uid=item.tag.tag,
                lobster_traces=refs,
                given=gwt[0],
                when=gwt[1],
                then=gwt[2],
                gtest_tag=item.tag.tag,
            )
        )

    return records


# ---------------------------------------------------------------------------
# Path resolution (action-sandbox vs runfiles contexts)
# ---------------------------------------------------------------------------


def resolve_path(raw: str) -> Path:
    """Resolve a Bazel-relative path to an absolute filesystem path.

    Resolution order:

    1. Absolute path that exists — returned as-is.
    2. Relative path that exists from CWD — works in Bazel action sandboxes
       where CWD is the execroot and short paths are available.
    3. ``$RUNFILES_DIR/<raw>`` — used by ``bazel run`` executables whose CWD
       is ``$BUILD_WORKSPACE_DIRECTORY``, not the execroot.
    4. Raw path returned unchanged as a last resort (caller will get a clear
       ``FileNotFoundError`` rather than a silent wrong-path write).
    """
    candidate = Path(raw)
    if candidate.is_absolute() and candidate.exists():
        return candidate
    if not candidate.is_absolute() and candidate.exists():
        return candidate.resolve()
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir and not candidate.is_absolute():
        via_runfiles = Path(runfiles_dir) / raw
        if via_runfiles.exists():
            return via_runfiles
    return candidate


# ---------------------------------------------------------------------------
# Req-ID extraction from lobster-req-trace manifests
# ---------------------------------------------------------------------------

# Only these TRLC requirement kinds belong in the coverage lock file.
# Feature requirements and assumed-system requirements are excluded — they
# are traceability targets, not directly testable component-level items.
_COMP_REQ_KINDS: frozenset[str] = frozenset({"CompReq"})


def read_req_metadata_from_lobster_files(
    lobster_manifest_path: Path,
) -> list[RequirementMeta]:
    """Parse a single-column manifest of lobster paths and return CompReq metadata.

    Each line is a path to a ``lobster-req-trace`` JSON file.  Only items
    with ``kind == "CompReq"`` are included; FeatReq and AssumedSystemReq
    items are silently skipped.

    Returns a deduplicated list sorted by requirement ID.  Each entry carries:

    * ``id`` — requirement identifier (tag without ``@version``)
    * ``version`` — version string from the ``@version`` tag suffix (e.g. ``"1"``)
    * ``description`` — requirement text from the ``text`` field
    """
    metadata: list[RequirementMeta] = []
    seen: dict[str, RequirementMeta] = {}

    for line in lobster_manifest_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        lobster_path = resolve_path(line)
        mh = Message_Handler()
        items: dict = {}
        try:
            lobster_read(mh, str(lobster_path), "req", items)
        except (OSError, LOBSTER_Error) as exc:
            raise ValueError(f"Cannot read lobster file {lobster_path}: {exc}") from exc

        for item in items.values():
            if not isinstance(item, Requirement):
                continue
            if item.kind not in _COMP_REQ_KINDS:
                continue  # skip FeatReq, AssumedSystemReq, etc.
            if item.tag.namespace != "req":
                continue
            req_id = item.tag.tag
            meta = RequirementMeta(
                id=req_id,
                version=str(item.tag.version) if item.tag.version is not None else "",
                description=str(item.text or ""),
            )
            existing = seen.get(req_id)
            if existing is not None:
                # Duplicate requirement ID across lobster files: keep the first
                # occurrence, but only warn when the duplicate actually
                # disagrees (version/description) with it — an identical
                # repeat (e.g. the same requirement reachable via two paths)
                # is expected and not worth flagging.
                if (existing.version, existing.description) != (meta.version, meta.description):
                    logger.warning(
                        "Requirement %r found more than once with differing "
                        "metadata (version %r vs %r); keeping the first occurrence.",
                        req_id,
                        existing.version,
                        meta.version,
                    )
                continue
            seen[req_id] = meta
            metadata.append(meta)

    return sorted(metadata, key=lambda m: m.id)


# ---------------------------------------------------------------------------
# Gtest scan — group records by requirement ID
# ---------------------------------------------------------------------------


def scan_gtest_lobster(
    gtest_lobster_path: Path,
    req_ids: list[str],
    package: str = "",
    label: str = "",
) -> dict[str, list[TestRecord]]:
    """Read a gtest.lobster file and group test records by requirement ID.

    Only records whose ``lobster_traces`` intersect with *req_ids* are kept.
    A record that covers multiple requirements appears in all their lists.
    Any traced ID that does *not* match a known CompReq (typo, or a trace to
    a FeatReq/AssumedSystemReq instead of a CompReq) is dropped and reported
    via a ``WARNING`` on stderr, instead of silently disappearing.

    The ``package`` (e.g. ``//score/message_passing``) is prepended to each
    uid to make it globally unique across components.  Pass
    ``"//" + ctx.label.package`` from the Bazel rule via the
    ``TEST_CASE_COVERAGE_PACKAGE`` environment variable.

    Args:
        label: Bazel label of the calling test_case_coverage target, used
               only to prefix warning messages.
    """
    all_records = read_gtest_lobster(gtest_lobster_path)
    req_id_set = set(req_ids)
    by_req: dict[str, list[TestRecord]] = {rid: [] for rid in req_ids}
    unmatched_traces: set[str] = set()
    for record in all_records:
        if package:
            # ``package`` is already "//" for a root-package target (no
            # trailing package name), so don't add a second "/" or the uid
            # ends up as "///Foo:GetNumber" instead of "//Foo:GetNumber".
            sep = "" if package.endswith("/") else "/"
            record = replace(record, uid=f"{package}{sep}{record.uid}")
        for trace in record.lobster_traces:
            if trace in req_id_set:
                by_req[trace].append(record)
            else:
                unmatched_traces.add(trace)

    for trace in sorted(unmatched_traces):
        logger.warning(
            "[%s] test case(s) trace to %r, which is not a known CompReq "
            "requirement ID for this component (typo, or tracing to a "
            "non-CompReq requirement) \u2014 this coverage is not recorded.",
            label,
            trace,
        )

    return by_req
