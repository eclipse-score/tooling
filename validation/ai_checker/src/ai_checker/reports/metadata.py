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
"""Helpers for report metadata (git hash, timestamp)."""

import os
import subprocess
from datetime import datetime


def get_git_hash() -> str:
    """Return the current git commit hash (8 chars) or 'Unknown'.

    Prefers the Bazel workspace-status stamp variables injected via
    ``--workspace_status_command``; falls back to ``git rev-parse HEAD`` in the
    source tree, and finally 'Unknown' (e.g. inside a hermetic Bazel action
    without git access).
    """
    for env_var in ("STABLE_GIT_COMMIT", "BUILD_EMBED_LABEL", "GIT_COMMIT"):
        value = os.environ.get(env_var, "").strip()
        if value:
            return value[:8]

    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=5,
            cwd=os.path.dirname(os.path.abspath(__file__)),
        )
        if result.returncode == 0:
            return result.stdout.strip()[:8]
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
        pass
    return "Unknown"


def get_timestamp() -> str:
    """Return the current timestamp as an ISO string (seconds precision)."""
    return datetime.now().isoformat(timespec="seconds")
