# *******************************************************************************
# Copyright (c) 2024 Contributors to the Eclipse Foundation
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

"""Shared helpers for the cr_checker test suite (unit and integration tests).

Kept in one place so both `test_cr_checker.py` (unit tests) and
`test_cr_checker_integration.py` (integration tests) reuse the same module
loading, template loading, and fixture-writing logic instead of each
reimplementing it.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
from pathlib import Path

TESTS_DIR = Path(__file__).resolve().parent
PACKAGE_DIR = TESTS_DIR.parent
TOOL_MODULE_PATH = PACKAGE_DIR / "tool" / "cr_checker.py"

# The real, production resource files shipped with the tool. Integration
# tests exercise these directly instead of ad-hoc fixtures, so the test suite
# validates the same configuration used by consumers of the tool.
RESOURCES_DIR = PACKAGE_DIR / "resources"
REAL_TEMPLATES_FILE = RESOURCES_DIR / "templates.ini"
REAL_CONFIG_FILE = RESOURCES_DIR / "config.json"
REAL_EXCLUSION_FILE = RESOURCES_DIR / "exclusion.txt"


def load_cr_checker_module():
    """Loads the cr_checker tool module directly from source."""
    spec = importlib.util.spec_from_file_location("cr_checker_module", TOOL_MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Failed to load cr_checker module from {TOOL_MODULE_PATH}")

    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_template(extension: str) -> str:
    """Returns the real copyright template configured for `extension`."""
    cr_checker = load_cr_checker_module()
    templates = cr_checker.load_templates(REAL_TEMPLATES_FILE)
    return templates[extension]


def write_config(path: Path, author: str) -> Path:
    """Writes a config.json with the given author under `path`.

    Written per-test (rather than reused directly from resources/config.json)
    so that the author can be varied, and so tests don't depend on the
    default author staying "Contributors to the Eclipse Foundation".
    """
    config_path = path / "config.json"
    config_path.write_text(json.dumps({"author": author}), encoding="utf-8")
    return config_path


def init_git_repo(path: Path) -> None:
    """Initializes a throwaway git repository at `path`.

    Needed by tests that exercise `collect_inputs`/`list_tracked_files`,
    which discover files via `git ls-files`.
    """
    subprocess.run(["git", "init", "-q"], cwd=path, check=True)
    subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=path, check=True)
    subprocess.run(["git", "config", "user.name", "Test"], cwd=path, check=True)
