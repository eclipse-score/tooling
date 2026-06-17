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
"""Shared error types and constants for the Copilot adapter."""

from __future__ import annotations

# Auth-related environment variables checked by the Copilot CLI (priority order)
AUTH_ENV_VARS: list[str] = [
    "COPILOT_GITHUB_TOKEN",  # Recommended for explicit Copilot usage
    "GH_TOKEN",  # GitHub CLI compatible
    "GITHUB_TOKEN",  # GitHub Actions compatible
]


class CopilotSetupError(RuntimeError):
    """Raised when the Copilot SDK environment is not correctly configured."""
