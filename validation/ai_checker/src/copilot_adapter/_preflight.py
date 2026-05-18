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
"""Pre-flight environment and authentication checks for the Copilot CLI."""

from __future__ import annotations

import logging
import os
import stat
from pathlib import Path
from typing import Optional

from ._errors import AUTH_ENV_VARS, CopilotSetupError

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# CLI binary
# ---------------------------------------------------------------------------


def resolve_copilot_cli_path() -> Optional[str]:
    """Find the executable copy of the copilot CLI created by copy_executables.

    rules_python strips the executable bit from binaries inside wheels.
    The pip.whl_mods / copy_executables mechanism creates an executable
    copy called ``copilot_cli`` next to the package.  We walk up from
    ``copilot.__file__`` until we find it.

    IMPORTANT: we must NOT resolve symlinks (Path.resolve()) because in
    the Bazel runfiles tree the symlinks point back to the source repo
    where the genrule output does not exist.  The raw __file__ path
    stays inside the execution root where the copy IS present.
    """
    import copilot as _copilot_pkg

    pkg_file = Path(_copilot_pkg.__file__)  # .../site-packages/copilot/__init__.py
    current = pkg_file.parent
    for _ in range(10):
        candidate = current / "copilot_cli"
        if candidate.exists():
            return str(candidate)
        current = current.parent
    return None


def check_cli_binary(cli_path: str) -> list[str]:
    """Validate that the CLI binary exists and is executable.

    Returns a list of problem descriptions (empty = all good).
    """
    problems: list[str] = []
    p = Path(cli_path)
    if not p.exists():
        problems.append(f"Copilot CLI binary not found at: {cli_path}")
        return problems
    if not p.is_file():
        problems.append(f"Copilot CLI path is not a file: {cli_path}")
        return problems
    mode = p.stat().st_mode
    if not (mode & stat.S_IXUSR):
        problems.append(
            f"Copilot CLI binary is NOT executable (mode {oct(mode)}): {cli_path}\n"
            "  Hint: rules_python strips +x from wheel binaries. Make sure\n"
            "  pip.whl_mods / copy_executables is configured in MODULE.bazel."
        )
    return problems


# ---------------------------------------------------------------------------
# Authentication sources
# ---------------------------------------------------------------------------


def check_auth_sources() -> list[str]:
    """Check that at least one authentication source is available.

    Returns a list of problem descriptions (empty = at least one source present).
    Hard-fails only when both token env vars AND $HOME are completely absent —
    the only case where the CLI is guaranteed to time-out authenticating.
    This check runs *before* the CLI is spawned to avoid wasting time.
    """
    for var in AUTH_ENV_VARS:
        if os.environ.get(var):
            return []  # A token is set — auth is possible

    if os.environ.get("HOME"):
        return []  # No token, but $HOME is set — stored OAuth may work

    return [
        "No authentication source is available for the Copilot CLI.\n"
        "  None of the token environment variables are set and $HOME is missing.\n"
        "  The Copilot CLI cannot authenticate without at least one of:\n"
        "    - $COPILOT_GITHUB_TOKEN  (recommended)\n"
        "    - $GH_TOKEN\n"
        "    - $GITHUB_TOKEN\n"
        "    - $HOME set so the CLI can read stored OAuth credentials\n"
        "  Fix: add --action_env=COPILOT_GITHUB_TOKEN to .bazelrc.ai_checker\n"
        "       and export COPILOT_GITHUB_TOKEN=<your-token> in your shell.\n"
        "  See: https://github.com/github/copilot-sdk/blob/main/docs/auth/index.md"
    ]


def describe_auth_sources() -> str:
    """Return a human-readable summary of all available auth sources."""
    lines = ["Authentication sources detected:"]
    found_any = False

    for var in AUTH_ENV_VARS:
        val = os.environ.get(var)
        if val:
            masked = val[:4] + "..." + val[-4:] if len(val) > 10 else "****"
            lines.append(f"  [OK] ${var} = {masked}")
            found_any = True
        else:
            lines.append(f"  [  ] ${var} — not set")

    home = os.environ.get("HOME", "")
    if home:
        lines.append(f"  [OK] $HOME = {home}  (CLI can search system keychain)")
    else:
        lines.append(
            "  [  ] $HOME — not set  (CLI cannot find stored OAuth credentials)"
        )

    if not found_any and not home:
        lines.append("")
        lines.append("  ** No authentication source available! **")
        lines.append(
            "  Fix: set COPILOT_GITHUB_TOKEN, or ensure HOME is passed to the action."
        )
        lines.append(
            "  See: https://github.com/github/copilot-sdk/blob/main/docs/auth/index.md"
        )

    lines.append("")
    proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy")
    if proxy:
        lines.append(f"  [OK] HTTPS_PROXY = {proxy}")
    else:
        lines.append(
            "  [  ] HTTPS_PROXY — not set  (may cause 'fetch failed' behind a proxy)"
        )

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Runtime environment
# ---------------------------------------------------------------------------


def check_environment() -> list[str]:
    """Check that the runtime environment has what the Copilot CLI needs.

    Returns a list of problem descriptions (empty = all good).
    These are warnings, not hard failures — a token env var may still work
    even when $HOME or HTTPS_PROXY are missing.
    """
    problems: list[str] = []

    if not os.environ.get("HOME"):
        problems.append(
            "HOME environment variable is not set.\n"
            "  The Copilot CLI needs HOME to locate stored OAuth credentials.\n"
            "  Ensure .bazelrc.ai_checker contains:  build --action_env=HOME"
        )

    if not os.environ.get("HTTPS_PROXY") and not os.environ.get("https_proxy"):
        problems.append(
            "HTTPS_PROXY / https_proxy environment variable is not set.\n"
            "  If you are behind a corporate proxy the Copilot CLI cannot\n"
            "  reach api.github.com and will fail with 'TypeError: fetch failed'.\n"
            "  Ensure .bazelrc.ai_checker contains:  build --action_env=HTTPS_PROXY"
        )

    return problems
