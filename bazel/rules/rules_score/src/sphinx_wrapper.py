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
"""
Wrapper script for running Sphinx builds in Bazel environments.

This script provides a command-line interface to Sphinx documentation builds,
handling argument parsing, environment configuration, and build execution.
It's designed to be used as part of Bazel build rules for Score modules.
"""

import argparse
import logging
import os
import re
import sys
import time
from contextlib import redirect_stdout, redirect_stderr
from pathlib import Path
from typing import List

from sphinx.cmd.build import main as sphinx_main

# Constants
DEFAULT_SOURCE_DIR = "."

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

# Mapping from --log-level to sphinx-build verbosity flags.
# warn  → -q  (suppress info; show warnings/errors only)
# info  → (nothing; sphinx default output)
# debug → -vv (verbose sphinx output)
_SPHINX_VERBOSITY_FLAGS = {
    "error": ["-Q"],
    "warn": ["-q"],
    "info": [],
    "debug": ["-vv"],
}

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


def build_sphinx_arguments(args: argparse.Namespace, extra_args: List[str] = None) -> List[str]:
    """
    Build the argument list for Sphinx.

    Args:
        args: Parsed command-line arguments
        extra_args: Additional arguments to forward to Sphinx (e.g., -D options from extra_opts)

    Returns:
        List of arguments to pass to Sphinx
    """
    source_dir = str(Path(args.index_file).parent) if args.index_file else DEFAULT_SOURCE_DIR
    config_dir = str(Path(args.config).parent) if args.config else source_dir

    base_arguments = [
        source_dir,  # source dir
        args.output_dir,  # output dir
        "-c",
        config_dir,  # config directory
        # "-W",                # treat warning as errors - disabled for modular builds
        # --keep-going is intentionally omitted: it only has an effect combined
        # with -W (report all errors before exiting instead of just the first),
        # so it is a no-op while -W stays disabled above.
        "-T",  # show details in case of errors in extensions
        "--jobs",
        "auto",
    ]

    base_arguments.extend(["-b", args.builder])

    # Apply sphinx-build verbosity flags derived from --log-level
    sphinx_verbosity = _SPHINX_VERBOSITY_FLAGS.get(getattr(args, "log_level", "warn"), [])
    base_arguments.extend(sphinx_verbosity)

    # Forward extra options (e.g., -D flags) to Sphinx
    if extra_args:
        base_arguments.extend(extra_args)

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
    except Exception:
        logger.exception("Sphinx build failed with exception")
        return 1

    end_time = time.perf_counter()
    duration = end_time - start_time

    if exit_code == 0:
        logger.info(f"docs ({builder}) finished successfully in {duration:.1f} seconds")
    else:
        logger.error(f"docs ({builder}) failed with exit code {exit_code} after {duration:.1f} seconds")

    return exit_code


def parse_arguments() -> argparse.Namespace:
    """
    Parse command-line arguments.

    Returns:
        Parsed command-line arguments
    """
    parser = argparse.ArgumentParser(description="Wrapper for Sphinx documentation builds in Bazel environments")

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
        "--log-level",
        choices=["error", "warn", "info", "debug"],
        default="warn",
        dest="log_level",
        help="Log level for wrapper and sphinx-build output (default: warn).",
    )

    return parser.parse_known_args()


def main() -> int:
    """
    Main entry point for the Sphinx wrapper script.

    Returns:
        Exit code (0 for success, non-zero for failure)
    """
    try:
        args, extra_args = parse_arguments()
        logging.basicConfig(level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s")
        validate_arguments(args)
        # Resolve execroot-relative tool paths to absolute paths NOW, while cwd
        # is still the execroot (Bazel guarantees cwd = execroot at action start).
        # Sphinx changes its working directory to the source/staging directory
        # before evaluating conf.py, so os.path.abspath() inside conf.py would
        # resolve against the wrong base.  Converting here is safe and means
        # conf.py receives already-absolute values via the environment.
        for _tool_var in ("GRAPHVIZ_DOT", "PLANTUML_BIN", "FTA_METAMODEL_DIR"):
            _tool_path = os.environ.get(_tool_var)
            if _tool_path and not os.path.isabs(_tool_path):
                os.environ[_tool_var] = os.path.abspath(_tool_path)
        # Create processor instance
        stdout_processor = StdoutProcessor()
        stderr_processor = StderrProcessor()
        # Redirect stdout and stderr
        with redirect_stderr(stderr_processor), redirect_stdout(stdout_processor):
            sphinx_args = build_sphinx_arguments(args, extra_args)
            exit_code = run_sphinx_build(sphinx_args, args.builder)
        return exit_code
    except ValueError as e:
        logger.error(f"Validation error: {e}")
        return 1
    except Exception:
        logger.exception("Unexpected error")
        return 1


if __name__ == "__main__":
    sys.exit(main())
