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

Thin shim around `@rules_python//sphinxdocs/private:sphinx_build.py`: this
module keeps only what upstream's sphinx-build entry point doesn't already do
-- the hermetic tool-env abspath fixup needed before Sphinx starts, and
one-shot stdout/stderr prefixing. Persistent-worker support (the Worker
class, digest diffing, the JSON worker protocol, retry-on-exit-code-2) is
loaded directly from rules_python at runtime, not ported/copied.
sphinx_module.bzl builds the full sphinx-build CLI (source dir, output dir,
-c/-b/... flags) itself, so this wrapper never parses its own custom flags.
"""

import importlib.util
import logging
import os
import re
import sys
import time
from contextlib import redirect_stderr, redirect_stdout
from typing import List, Optional

from python.runfiles import runfiles
from sphinx.cmd.build import main as sphinx_main

logger = logging.getLogger(__name__)

SANDBOX_PATH = re.compile(r"^.*_main/")

# Env vars sphinx_module.bzl's _hermetic_tool_env() passes as execroot-relative
# paths; conf.py needs them absolute since Sphinx chdirs into confdir first.
_HERMETIC_TOOL_ENV_VARS = ("GRAPHVIZ_DOT", "PLANTUML_BIN", "FTA_METAMODEL_DIR")

# Runfiles path of upstream's sphinx-build entry point, using the *apparent*
# repo name ("rules_python", as declared in this repo's own MODULE.bazel) so
# Rlocation() resolves it via repo mapping regardless of bzlmod's canonical
# repo-name format.
_SPHINX_BUILD_RLOCATION = "rules_python/sphinxdocs/private/sphinx_build.py"


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


def fixup_hermetic_tool_env() -> None:
    """
    Resolve execroot-relative tool paths to absolute paths.

    Must run now, while cwd is still the execroot (Bazel guarantees cwd ==
    execroot at process start) -- Sphinx changes its working directory to
    the confdir before evaluating conf.py, so os.path.abspath() calls inside
    conf.py itself would resolve against the wrong base. Runs once per
    worker process too, since these are tool paths, not per-request data.
    """
    for var in _HERMETIC_TOOL_ENV_VARS:
        value = os.environ.get(var)
        if value and not os.path.isabs(value):
            os.environ[var] = os.path.abspath(value)


def expand_param_files(argv: List[str]) -> List[str]:
    """
    Expand `@file` tokens, one argument per non-blank line.

    sphinx_module.bzl always builds its Args with `use_param_file(...,
    use_always=True)`, so a one-shot invocation's argv is always a single
    `@file` token; persistent-worker requests arrive already expanded via
    the JSON protocol and never go through this function.
    """
    expanded = []
    for arg in argv:
        if arg.startswith("@"):
            with open(arg[1:]) as fp:
                expanded.extend(line.strip() for line in fp if line.strip())
        else:
            expanded.append(arg)
    return expanded


def _load_sphinx_build_module():
    """
    Load rules_python's sphinx_build.py module by its runfiles path.

    Loaded at runtime (not ported/copied) so this repo depends directly on
    upstream's Worker/persistent-worker-protocol implementation instead of
    maintaining a local copy of it.
    """
    r = runfiles.Create()
    path = r.Rlocation(_SPHINX_BUILD_RLOCATION)
    if not path:
        raise RuntimeError(f"Could not locate {_SPHINX_BUILD_RLOCATION} in runfiles")
    spec = importlib.util.spec_from_file_location("sphinx_build", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules["sphinx_build"] = module
    spec.loader.exec_module(module)
    return module


def run_persistent_worker() -> int:
    """Hand off to upstream's Worker for the lifetime of this process."""
    sphinx_build = _load_sphinx_build_module()
    with sphinx_build.Worker(sys.stdin, sys.stdout, os.getcwd()) as worker:
        worker.run()
    return 0


def _infer_wrapper_log_level(sphinx_args: List[str]) -> int:
    """Derive this wrapper's own logging level from sphinx-build's verbosity flags.

    Mirrors sphinx_module.bzl's `_SPHINX_VERBOSITY_FLAGS` mapping. Reads flags
    sphinx_module.bzl already adds for sphinx-build's own consumption rather
    than parsing a new flag of this wrapper's own, so the verbosity build
    setting keeps controlling the wrapper's diagnostics (build duration,
    computed args) too, not just sphinx-build's own output.
    """
    if "-vvv" in sphinx_args or "-vv" in sphinx_args:
        return logging.DEBUG
    if "-q" in sphinx_args:
        return logging.WARNING
    return logging.INFO


def run_one_shot(sphinx_args: List[str]) -> int:
    """Run a single Sphinx build, with prefixed/sandbox-stripped stdout/stderr."""
    logging.getLogger().setLevel(_infer_wrapper_log_level(sphinx_args))
    logger.debug(f"Sphinx arguments: {sphinx_args}")
    start_time = time.perf_counter()
    with redirect_stderr(StderrProcessor()), redirect_stdout(StdoutProcessor()):
        try:
            exit_code = sphinx_main(sphinx_args)
        except Exception:
            logger.exception("Sphinx build failed with exception")
            return 1
    duration = time.perf_counter() - start_time
    if exit_code == 0:
        logger.info(f"Sphinx build finished successfully in {duration:.1f} seconds")
    else:
        logger.error(f"Sphinx build failed with exit code {exit_code} after {duration:.1f} seconds")
    return exit_code


def main(argv: Optional[List[str]] = None) -> int:
    """
    Main entry point for the Sphinx wrapper script.

    Returns:
        Exit code (0 for success, non-zero for failure)
    """
    argv = sys.argv[1:] if argv is None else argv
    logging.basicConfig(level=logging.WARNING, format="%(levelname)s: %(message)s")
    fixup_hermetic_tool_env()
    if "--persistent_worker" in argv:
        return run_persistent_worker()
    return run_one_shot(expand_param_files(argv))


if __name__ == "__main__":
    sys.exit(main())
