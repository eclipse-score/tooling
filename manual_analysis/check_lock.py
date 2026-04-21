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
"""Checks whether committed and computed manual-analysis lock files match."""

from pathlib import Path


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def evaluate_lock_files(computed: Path, committed: Path) -> tuple[bool, str | None]:
    if not computed.exists():
        return False, f"computed lock file not found: {computed}"
    if not committed.exists():
        return False, f"committed lock file not found: {committed}"
    if _read(computed) == _read(committed):
        return True, None
    return False, "Manual analysis lock file is out of date."
