"""
Wrapper script for running Sphinx builds in Bazel environments.

This script provides a command-line interface to Sphinx documentation builds,
handling argument parsing, environment configuration, and build execution.
It's designed to be used as part of Bazel build rules for Score modules.
"""

import argparse
import logging
import os
import sys
import time
from pathlib import Path
from typing import List, Optional
import re
import sys
from contextlib import redirect_stdout, redirect_stderr

from sphinx.cmd.build import main as sphinx_main

# Constants
DEFAULT_PORT = 8000
DEFAULT_GITHUB_VERSION = "main"
DEFAULT_SOURCE_DIR = "."

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(levelname)s: %(message)s",
)
logger = logging.getLogger(__name__)

SANDBOX_PATH = re.compile(r"^.*_main/")


class StdoutProcessor:
    def write(self, text):
        if text.strip():
            text = re.sub(SANDBOX_PATH, "", text)
            sys.__stdout__.write(f"[SPHINX_STDOUT]: {text.strip()}\n")

    def flush(self):
        sys.__stdout__.flush()


class StderrProcessor:
    def write(self, text):
        if text.strip():
            text = re.sub(SANDBOX_PATH, "", text)
            sys.__stderr__.write(f"[SPHINX_STDERR]: {text.strip()}\n")

    def flush(self):
        sys.__stderr__.flush()


def get_env(name: str, required: bool = True) -> Optional[str]:
    """
    Get an environment variable value.

    Args:
        name: The name of the environment variable
        required: Whether the variable is required (raises error if not set)

    Returns:
        The value of the environment variable, or None if not required and not set

    Raises:
        ValueError: If the variable is required but not set
    """
    val = os.environ.get(name)
    logger.debug(f"Environment variable {name} = {val}")
    if val is None and required:
        raise ValueError(f"Required environment variable {name} is not set")
    return val


def validate_arguments(args: argparse.Namespace) -> None:
    """
    Validate required command-line arguments.

    Args:
        args: Parsed command-line arguments

    Raises:
        ValueError: If required arguments are missing or invalid
    """
    if not args.index_file:
        raise ValueError("--index_file is required")
    if not args.output_dir:
        raise ValueError("--output_dir is required")
    if not args.builder:
        raise ValueError("--builder is required")

    # Validate that index file exists if it's a real path
    index_path = Path(args.index_file)
    if not index_path.exists():
        raise ValueError(f"Index file does not exist: {args.index_file}")


def build_sphinx_arguments(args: argparse.Namespace) -> List[str]:
    """
    Build the argument list for Sphinx.

    Args:
        args: Parsed command-line arguments

    Returns:
        List of arguments to pass to Sphinx
    """
    source_dir = (
        str(Path(args.index_file).parent) if args.index_file else DEFAULT_SOURCE_DIR
    )
    config_dir = str(Path(args.config).parent) if args.config else source_dir

    base_arguments = [
        source_dir,  # source dir
        args.output_dir,  # output dir
        "-c",
        config_dir,  # config directory
        # "-W",                # treat warning as errors - disabled for modular builds
        "--keep-going",  # do not abort after one error
        "-T",  # show details in case of errors in extensions
        "--jobs",
        "auto",
    ]

    # Configure sphinx build with GitHub user and repo from CLI
    if args.github_user and args.github_repo:
        base_arguments.extend(
            [
                f"-A=github_user={args.github_user}",
                f"-A=github_repo={args.github_repo}",
                f"-A=github_version={DEFAULT_GITHUB_VERSION}",
            ]
        )

        # Add doc_path if SOURCE_DIRECTORY environment variable is set
        source_directory = get_env("SOURCE_DIRECTORY", required=False)
        if source_directory:
            base_arguments.append(f"-A=doc_path='{source_directory}'")

    base_arguments.extend(["-b", args.builder])

    return base_arguments


def run_sphinx_build(sphinx_args: List[str], builder: str) -> int:
    """
    Execute the Sphinx build and measure duration.

    Args:
        sphinx_args: Arguments to pass to Sphinx
        builder: The builder type (for logging purposes)

    Returns:
        The exit code from Sphinx build
    """
    logger.info(f"Starting Sphinx build with builder: {builder}")
    logger.debug(f"Sphinx arguments: {sphinx_args}")

    start_time = time.perf_counter()

    try:
        exit_code = sphinx_main(sphinx_args)
    except Exception as e:
        logger.error(f"Sphinx build failed with exception: {e}")
        return 1

    end_time = time.perf_counter()
    duration = end_time - start_time

    if exit_code == 0:
        logger.info(f"docs ({builder}) finished successfully in {duration:.1f} seconds")
    else:
        logger.error(
            f"docs ({builder}) failed with exit code {exit_code} after {duration:.1f} seconds"
        )

    return exit_code


def parse_arguments() -> argparse.Namespace:
    """
    Parse command-line arguments.

    Returns:
        Parsed command-line arguments
    """
    parser = argparse.ArgumentParser(
        description="Wrapper for Sphinx documentation builds in Bazel environments"
    )

    # Required arguments
    parser.add_argument(
        "--index_file",
        required=True,
        help="Path to the index file (e.g., index.rst)",
    )
    parser.add_argument(
        "--output_dir",
        required=True,
        help="Build output directory",
    )
    parser.add_argument(
        "--builder",
        required=True,
        help="Sphinx builder to use (e.g., html, needs, json)",
    )

    # Optional arguments
    parser.add_argument(
        "--config",
        help="Path to config file (conf.py)",
    )
    parser.add_argument(
        "--github_user",
        help="GitHub username to embed in the Sphinx build",
    )
    parser.add_argument(
        "--github_repo",
        help="GitHub repository to embed in the Sphinx build",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=DEFAULT_PORT,
        help=f"Port to use for live preview (default: {DEFAULT_PORT}). Use 0 for auto-detection.",
    )

    return parser.parse_args()


def main() -> int:
    """
    Main entry point for the Sphinx wrapper script.

    Returns:
        Exit code (0 for success, non-zero for failure)
    """
    try:
        args = parse_arguments()
        validate_arguments(args)
        # Create processor instance
        stdout_processor = StdoutProcessor()
        stderr_processor = StderrProcessor()
        # Redirect stdout and stderr
        with redirect_stderr(stdout_processor), redirect_stdout(stderr_processor):
            sphinx_args = build_sphinx_arguments(args)
            exit_code = run_sphinx_build(sphinx_args, args.builder)
            exit_code = 0
        return exit_code
    except ValueError as e:
        logger.error(f"Validation error: {e}")
        return 1
    except Exception as e:
        logger.error(f"Unexpected error: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
