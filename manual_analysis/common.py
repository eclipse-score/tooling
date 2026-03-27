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
from importlib import import_module
from pathlib import Path


def _create_runfiles():  # type: ignore[no-untyped-def]
    runfiles_module = import_module("runfiles")
    runfiles_class = getattr(runfiles_module, "Runfiles")
    return runfiles_class.Create()


def resolve_path(raw_path: str) -> Path:
    """Resolve path from Bazel env/execpath style values."""
    candidate = Path(raw_path)

    try:
        runfiles = _create_runfiles()
    except (ImportError, AttributeError):
        runfiles = None
    if runfiles is not None:
        resolved_path = runfiles.Rlocation(str(candidate))
        if resolved_path:
            resolved = Path(resolved_path)
            if resolved.exists():
                return resolved

    return candidate
