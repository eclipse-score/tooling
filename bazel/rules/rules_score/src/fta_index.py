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
"""Reader of the ``fta_index.json`` that ``puml_cli`` writes in FTA mode.

The index holds the review evidence of the fault trees: the diagrams and their
source paths, the ``$FailureMode`` calls of each failure mode, the transfer
gates through which a root cause reaches a failure mode, and the completeness
findings (see ``puml_fta::render_index``).
"""

from __future__ import annotations

import dataclasses
import json

SUPPORTED_VERSION = 1


@dataclasses.dataclass(frozen=True)
class SourceRef:
    """A macro call of a diagram."""

    diagram: str
    line: int
    alias: str


@dataclasses.dataclass(frozen=True)
class Finding:
    """A spot where the fault trees are incomplete; ``line`` is 0 when unknown."""

    kind: str
    diagram: str
    line: int
    message: str


@dataclasses.dataclass(frozen=True)
class FtaIndex:
    """The content of ``fta_index.json``.

    Attributes:
        diagrams: Diagram basename to the path of its source file.
        failure_mode_trees: Failure mode fqn to its ``$FailureMode`` calls.
        via: Root cause name to failure mode fqn to the transfer gates the root
            cause reaches the failure mode through.
        findings: The completeness findings.
    """

    diagrams: dict[str, str]
    failure_mode_trees: dict[str, list[SourceRef]]
    via: dict[str, dict[str, list[SourceRef]]]
    findings: list[Finding]

    @classmethod
    def empty(cls) -> FtaIndex:
        """The index of an analysis without fault-tree diagrams."""
        return cls(diagrams={}, failure_mode_trees={}, via={}, findings=[])

    @classmethod
    def load(cls, path: str) -> FtaIndex:
        """Read an index file.

        Raises:
            ValueError: If the file is not an index of the supported version.
        """
        with open(path, encoding="utf-8") as fh:
            data = json.load(fh)
        version = data.get("version") if isinstance(data, dict) else None
        if version != SUPPORTED_VERSION:
            raise ValueError(f"{path}: unsupported fta_index version {version!r}, expected {SUPPORTED_VERSION}")
        return cls(
            diagrams=dict(data["diagrams"]),
            failure_mode_trees={fqn: _refs(refs) for fqn, refs in data["failure_mode_trees"].items()},
            via={
                root_cause: {fqn: _refs(refs) for fqn, refs in per_failure_mode.items()}
                for root_cause, per_failure_mode in data["via"].items()
            },
            findings=[Finding(**finding) for finding in data["findings"]],
        )


def _refs(refs: list[dict]) -> list[SourceRef]:
    return [SourceRef(**ref) for ref in refs]
