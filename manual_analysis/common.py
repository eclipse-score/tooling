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
from runfiles import Runfiles


def _create_runfiles() -> Runfiles:
    return Runfiles.Create()


def resolve_path(raw_path: str) -> Path:
    """Resolve path from Bazel env/execpath style values."""
    candidate = Path(raw_path)

    runfiles = _create_runfiles()
    resolved_path = runfiles.Rlocation(str(candidate))
    if resolved_path:
        resolved = Path(resolved_path)
        if resolved.exists():
            return resolved

    return candidate
