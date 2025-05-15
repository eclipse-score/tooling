"""Path helpers for running python code within Bazel."""

import os
from pathlib import Path


def runfiles_dir() -> Path:
    """Returns the runfiles directory for the current Bazel build."""
    ...
    
def bazel_root() -> Path:
    """
    Returns the location of MODULE.bazel file.
    TODO: which one?
    Only works when called from bazel.
    """

    env_root = os.getenv("BUILD_WORKSPACE_DIRECTORY")
    if env_root:
        return Path(env_root).resolve()
    else:
        return None


def git_root() -> Path:
    """Returns a path to the git repository."""
    return _find_upwards(Path(__file__).resolve(), marker=".git")


def cwd() -> Path:
    """Returns the current working directory = invocation directory."""
    return Path(os.getenv("BUILD_WORKING_DIRECTORY") or os.getcwd()).resolve()


def _find_upwards(start: Path, marker: str) -> Path:
    """
    Walks up from `start` to find a directory containing `marker`.
    Raises FileNotFoundError if not found.
    """
    for parent in [start] + list(start.parents):
        if (parent / marker).exists():
            return parent
    raise FileNotFoundError(f"Could not find '{marker}' in any parent directory of {start}")
