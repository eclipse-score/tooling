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
"""Lifecycle management for the Copilot CLI subprocess and SDK client."""

from __future__ import annotations

import logging
from typing import Any, Optional

from copilot import CopilotClient, SubprocessConfig

from ._errors import CopilotSetupError
from ._preflight import (
    check_auth_sources,
    check_cli_binary,
    check_environment,
    describe_auth_sources,
    resolve_copilot_cli_path,
)

logger = logging.getLogger(__name__)


class CopilotClientManager:
    """Owns the lifecycle of a single CopilotClient / CLI subprocess.

    Responsibilities:
    - Resolve the CLI binary path (rules_python copy_executables workaround)
    - Run pre-flight checks before spawning the process
    - Start the subprocess and verify authentication
    - Expose the live client for callers
    - Shut the process down cleanly on close

    This class is intentionally not a Pydantic model — it holds mutable
    runtime state that must not be serialised.
    """

    def __init__(self, copilot_client_options: dict[str, Any] | None = None) -> None:
        self._options: dict[str, Any] = dict(copilot_client_options or {})
        self._client: Optional[CopilotClient] = None
        self._started: bool = False

    # ------------------------------------------------------------------
    # Public interface
    # ------------------------------------------------------------------

    async def ensure_client(self) -> CopilotClient:
        """Return a started, authenticated CopilotClient.

        Creates and starts the client on the first call; subsequent calls
        return the cached instance immediately.

        Pre-flight sequence (runs once, before the CLI is spawned):
        1. Resolve the CLI binary path
        2. Validate the binary exists and is executable
        3. Hard-fail if no auth source is available at all
        4. Warn about missing $HOME / HTTPS_PROXY (non-fatal)
        5. Start the CLI subprocess
        6. Verify authentication via get_auth_status()

        Raises:
            CopilotSetupError: With a detailed, actionable message for any
                failure that prevents the CLI from being used.
        """
        if self._client is None:
            self._client = self._create_client()

        if not self._started:
            await self._start_and_verify()

        return self._client

    async def close(self) -> None:
        """Stop the CLI subprocess if it is running."""
        if self._client and self._started:
            await self._client.stop()
            self._started = False

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _create_client(self) -> CopilotClient:
        """Run pre-flight checks and construct (but do not start) the client."""
        opts = dict(self._options)

        # --- Resolve CLI binary path ----------------------------------
        if "cli_path" not in opts and "cli_url" not in opts:
            resolved = resolve_copilot_cli_path()
            if resolved:
                opts["cli_path"] = resolved
                logger.info("Resolved Copilot CLI path: %s", resolved)
            else:
                logger.warning(
                    "Could not find copilot_cli (copy_executables target). "
                    "Falling back to bundled binary — this may fail with "
                    "PermissionError if the executable bit was stripped."
                )

        # --- Check binary --------------------------------------------
        cli_path = opts.get("cli_path")
        if cli_path:
            problems = check_cli_binary(cli_path)
            if problems:
                raise CopilotSetupError(
                    "Copilot CLI binary check failed:\n"
                    + "\n".join(f"  - {p}" for p in problems)
                )

        # --- Hard-fail if no auth source available -------------------
        auth_problems = check_auth_sources()
        if auth_problems:
            raise CopilotSetupError(
                "Copilot authentication pre-flight check failed — "
                "the CLI process will not be started:\n"
                + "\n".join(f"  - {p}" for p in auth_problems)
                + "\n\n"
                + describe_auth_sources()
            )

        # --- Warn about non-fatal env issues -------------------------
        env_problems = check_environment()
        if env_problems:
            logger.warning(
                "Environment issues detected:\n%s\n%s",
                "\n".join(f"  - {p}" for p in env_problems),
                describe_auth_sources(),
            )

        logger.info("Starting CopilotClient...\n%s", describe_auth_sources())
        _subprocess_fields = frozenset(
            {
                "cli_path",
                "cli_args",
                "cwd",
                "use_stdio",
                "port",
                "log_level",
                "env",
                "github_token",
                "use_logged_in_user",
                "telemetry",
                "session_fs",
                "session_idle_timeout_seconds",
            }
        )
        subprocess_kwargs = {k: v for k, v in opts.items() if k in _subprocess_fields}
        return CopilotClient(SubprocessConfig(**subprocess_kwargs))

    async def _start_and_verify(self) -> None:
        """Start the CLI subprocess and verify authentication."""
        assert self._client is not None

        try:
            await self._client.start()
        except PermissionError as exc:
            raise CopilotSetupError(
                f"PermissionError starting Copilot CLI: {exc}\n"
                "  The CLI binary is not executable. Make sure\n"
                "  pip.whl_mods / copy_executables is configured in MODULE.bazel\n"
                "  to create an executable copy of copilot/bin/copilot."
            ) from exc
        except RuntimeError as exc:
            if "timeout" in str(exc).lower() or "Timeout" in str(exc):
                raise CopilotSetupError(
                    f"Timeout starting Copilot CLI server: {exc}\n"
                    "  The CLI started but did not become ready in time.\n"
                    "  This usually means the CLI cannot authenticate.\n\n"
                    + describe_auth_sources()
                    + "\n\n"
                    "  Possible fixes:\n"
                    "  1. Run 'copilot' in a terminal and sign in interactively.\n"
                    "  2. Set COPILOT_GITHUB_TOKEN (or GH_TOKEN / GITHUB_TOKEN)\n"
                    "     and pass it via --action_env=COPILOT_GITHUB_TOKEN.\n"
                    "  3. Ensure HOME is available in the action environment\n"
                    "     (use_default_shell_env = True in the Bazel rule).\n"
                    "  See: https://github.com/github/copilot-sdk/blob/main/docs/auth/index.md"
                ) from exc
            raise
        except Exception as exc:
            raise CopilotSetupError(
                f"Failed to start CopilotClient: {type(exc).__name__}: {exc}\n\n"
                + describe_auth_sources()
            ) from exc

        self._started = True
        await self._verify_auth()

    async def _verify_auth(self) -> None:
        """Log the result of get_auth_status() as a diagnostic; never hard-fail.

        Rationale: get_auth_status() can return isAuthenticated=False even when
        the CLI is fully functional — for example:
        - The auth state is resolved lazily on the first real request.
        - GitHub Enterprise hosts (*.ghe.com) may not be reflected immediately.
        - There is a brief window after start() where the status is not yet set.

        A false-positive hard-fail here would block valid requests.  The actual
        LLM call (send_and_wait) will fail with a clear error if auth is truly
        broken, so we demote this check to a warning-only diagnostic.
        """
        assert self._client is not None
        try:
            auth_status = await self._client.get_auth_status()
            # The SDK uses camelCase on some versions, snake_case on others.
            is_auth = getattr(auth_status, "isAuthenticated", None) or getattr(
                auth_status, "is_authenticated", None
            )
            if is_auth:
                user = getattr(auth_status, "login", "unknown")
                logger.info("Copilot authenticated as: %s", user)
            else:
                # Log as a warning only — do not raise.  The CLI may still work.
                logger.warning(
                    "get_auth_status() reports isAuthenticated=False — "
                    "continuing anyway; auth may be resolved on first request.\n"
                    "  Auth status: %s\n%s",
                    auth_status,
                    describe_auth_sources(),
                )
        except Exception as exc:
            # get_auth_status itself failed — log but do not block.
            logger.warning(
                "Could not verify auth status (non-fatal): %s: %s",
                type(exc).__name__,
                exc,
            )
