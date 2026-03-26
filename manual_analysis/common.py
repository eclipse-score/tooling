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
import os
from pathlib import Path


def resolve_path(raw_path: str) -> Path:
    """Resolve path from Bazel env/execpath style values."""
    candidate = Path(raw_path)
    if candidate.is_absolute() and candidate.exists():
        return candidate
    if candidate.exists():
        return candidate.resolve()

    base = os.environ.get("BUILD_WORKING_DIRECTORY")
    if base:
        resolved = Path(base) / candidate
        if resolved.exists():
            return resolved

    return candidate
