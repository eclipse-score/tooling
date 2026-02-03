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

"""
Generic Sphinx configuration template for SCORE modules.

This file is auto-generated from a template and should not be edited directly.
Template variables like {PROJECT_NAME} are replaced during Bazel build.
"""

import json
import os
from pathlib import Path
from typing import Any, Dict, List
from sphinx.util import logging


# Create a logger with the Sphinx namespace
logger = logging.getLogger(__name__)

# Project configuration - {PROJECT_NAME} will be replaced by the module name during build
project = "{PROJECT_NAME}"
author = "S-CORE"
version = "1.0"
release = "1.0.0"
project_url = (
    "https://github.com/eclipse-score"  # Required by score_metamodel extension
)

# Sphinx extensions - comprehensive list for SCORE modules
extensions = [
    "sphinx_needs",
    "sphinx_design",
    "myst_parser",
    "sphinxcontrib.plantuml",
    "score_plantuml",
    "score_metamodel",
    "score_draw_uml_funcs",
    "score_source_code_linker",
    "score_layout",
]

# MyST parser extensions
myst_enable_extensions = ["colon_fence"]

# Exclude patterns for Bazel builds
exclude_patterns = [
    "bazel-*",
    ".venv*",
]

# Enable markdown rendering
source_suffix = {
    ".rst": "restructuredtext",
    ".md": "markdown",
}

# Enable numref for cross-references
numfig = True

# HTML theme
# html_theme = "pydata_sphinx_theme"


# Configuration constants
NEEDS_EXTERNAL_FILE = "needs_external_needs.json"
BAZEL_OUT_DIR = "bazel-out"


def find_workspace_root() -> Path:
    """
    Find the Bazel workspace root by looking for the bazel-out directory.

    Returns:
        Path to the workspace root directory
    """
    current = Path.cwd()

    # Traverse up the directory tree looking for bazel-out
    while current != current.parent:
        if (current / BAZEL_OUT_DIR).exists():
            return current
        current = current.parent

    # If we reach the root without finding it, return current directory
    return Path.cwd()


def load_external_needs() -> List[Dict[str, Any]]:
    """
    Load external needs configuration from JSON file.

    This function reads the needs_external_needs.json file if it exists and
    resolves relative paths to absolute paths based on the workspace root.

    Returns:
        List of external needs configurations with resolved paths
    """
    needs_file = Path(NEEDS_EXTERNAL_FILE)

    if not needs_file.exists():
        logger.info(f"{NEEDS_EXTERNAL_FILE} not found - no external dependencies")
        return []

    logger.info(f"Loading external needs from {NEEDS_EXTERNAL_FILE}")

    try:
        with needs_file.open("r", encoding="utf-8") as file:
            needs_dict = json.load(file)
    except json.JSONDecodeError as e:
        logger.error(f"Failed to parse {NEEDS_EXTERNAL_FILE}: {e}")
        return []
    except Exception as e:
        logger.error(f"Failed to read {NEEDS_EXTERNAL_FILE}: {e}")
        return []

    workspace_root = find_workspace_root()
    logger.info(f"Workspace root: {workspace_root}")

    external_needs = []
    for key, config in needs_dict.items():
        if "json_path" not in config:
            logger.warning(
                f"External needs config for '{key}' missing 'json_path', skipping"
            )
            continue

        # Resolve relative path to absolute path
        # Bazel provides relative paths like: bazel-out/k8-fastbuild/bin/.../needs.json
        # We need absolute paths: .../execroot/_main/bazel-out/...
        json_path = workspace_root / config["json_path"]
        config["json_path"] = str(json_path)

        logger.info(f"Added external needs config for '{key}':")
        logger.info(f"  json_path: {config['json_path']}")
        logger.info(f"  id_prefix: {config.get('id_prefix', 'none')}")
        logger.info(f"  version: {config.get('version', 'none')}")

        external_needs.append(config)

    return external_needs


def verify_config(app: Any, config: Any) -> None:
    """
    Initialize and verify external needs configuration.

    This is called during Sphinx's config-inited event to ensure
    external needs configuration is correctly set up. We need to
    explicitly set the config value here because Sphinx doesn't
    automatically pick up module-level variables for extension configs.

    Args:
        app: Sphinx application object
        config: Sphinx configuration object
    """
    # Set the config from our module-level variable
    # This is needed because sphinx-needs registers its config with add_config_value
    # which doesn't automatically pick up module-level variables from conf.py
    if needs_external_needs:
        config.needs_external_needs = needs_external_needs

    logger.info("=" * 80)
    logger.info("Verifying Sphinx configuration")
    logger.info(f"  Project: {config.project}")
    logger.info(f"  External needs count: {len(config.needs_external_needs)}")
    logger.info("=" * 80)


def setup(app: Any) -> Dict[str, Any]:
    """
    Sphinx setup hook to register event listeners.

    Args:
        app: Sphinx application object

    Returns:
        Extension metadata dictionary
    """
    app.connect("config-inited", verify_config)

    return {
        "version": "1.0",
        "parallel_read_safe": True,
        "parallel_write_safe": True,
    }


# Initialize external needs configuration
logger.info("=" * 80)
logger.info(f"Sphinx configuration loaded for project: {project}")
logger.info(f"Current working directory: {Path.cwd()}")

# Load external needs configuration
# Note: This sets a module-level variable that is then applied to the Sphinx
# config object in the verify_config callback during the config-inited event
needs_external_needs = load_external_needs()
