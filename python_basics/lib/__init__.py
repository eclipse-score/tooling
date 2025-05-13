# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
import logging
import os
import sys
from pathlib import Path

logger = logging.getLogger(__name__)

def print_env():
    """Prints the environment variables for debugging purposes."""
    print("Environment variables:")
    for key, value in os.environ.items():
        print(f"{key}: {value}")
    print("")

def find_git_root():
    print_env()

    git_root = Path(__file__).resolve()
    while not (git_root / ".git").exists():
        git_root = git_root.parent
        if git_root == Path("/"):
            sys.exit(
                "Could not find git root. Please run this script from the "
                + "root of the repository."
            )
    return git_root


def _get_runfiles_dir(
    cwd: Path,
    env_runfiles: Path | None,
    git_root: Path,
) -> Path:
    """Functional (and therefore testable) logic to determine the runfiles directory."""

    logger.debug(
        "_get_runfiles_dir(\n"
        + f"  {cwd=},\n"
        + f"  {env_runfiles=},\n"
        + f"  {git_root=}\n"
        + ")"
    )
    print_env()

    if env_runfiles:
        # Runfiles are only available when running in Bazel.
        # bazel build and bazel run are both supported.
        # i.e. `bazel build //docs:docs` and `bazel run //docs:incremental`.
        logger.debug("Using env[runfiles] to find the runfiles...")

        if env_runfiles.is_absolute():
            # In case of `bazel run` it will point to the global cache directory, which
            # has a new hash every time. And it's not pretty.
            # However `bazel-out` is a symlink to that same cache directory!
            parts = str(env_runfiles).split("/bazel-out/")
            if len(parts) != 2:
                # This will intentionally also fail if "bazel-out" appears multiple
                # times in the path. Will be fixed on demand only.
                sys.exit("Could not find bazel-out in runfiles path.")
            runfiles_dir = git_root / Path("bazel-out") / parts[1]
            logger.debug(f"Made runfiles dir pretty: {runfiles_dir}")
        else:
            runfiles_dir = git_root / env_runfiles

    else:
        # The only way to land here is when running from within the virtual
        # environment created by the `:ide_support` rule.
        # i.e. esbonio or manual sphinx-build execution within the virtual
        # environment.
        logger.debug("Running outside bazel.")

        print(f"{git_root=}")

        # TODO: "process-docs" is in SOURCE_DIR!!
        runfiles_dir = (
            Path(git_root) / "bazel-bin" / "process-docs" / "ide_support.runfiles"
        )

    return runfiles_dir


def get_runfiles_dir() -> Path:
    """Runfiles directory"""

    env_runfiles = os.getenv("RUNFILES_DIR")
    env_runfiles = Path(env_runfiles) if env_runfiles else None

    runfiles = _get_runfiles_dir(
        cwd=Path(os.getcwd()),
        env_runfiles=env_runfiles,
        git_root=find_git_root(),
    )

    if not runfiles.exists():
        sys.exit(f"Could not find runfiles at {runfiles}.")

    return runfiles
