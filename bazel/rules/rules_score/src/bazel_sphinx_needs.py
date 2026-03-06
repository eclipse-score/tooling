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
#
import json
import logging
import os
import sys
from pathlib import Path
from typing import Any, Dict, List

# Create a logger with the Sphinx namespace
logger = logging.getLogger(__name__)

# Configuration constants defined by bazel
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

        if "version" not in config:
            logger.warning(
                f"External needs config for '{key}' missing 'version', skipping"
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
        logger.info(f"  version: {config['version']}")

        external_needs.append(config)

    return external_needs


def log_config_info(project_name: str) -> None:
    """
    Log Sphinx configuration information.

    Args:
        project_name: Name of the Sphinx project
    """
    logger.info("=" * 80)
    logger.info(f"Sphinx configuration loaded for project: {project_name}")
    logger.info(f"Current working directory: {Path.cwd()}")
    logger.info("=" * 80)


def verify_config(
    app: Any, config: Any, needs_external_needs: List[Dict[str, Any]]
) -> None:
    """
    Verify and update Sphinx configuration with external needs.

    Args:
        app: Sphinx application instance
        config: Sphinx configuration object
        needs_external_needs: List of external needs configurations
    """
    config.needs_external_needs = needs_external_needs
    logger.info("=" * 80)
    logger.info("Verifying Sphinx configuration")
    logger.info(f"  Project: {config.project}")
    logger.info(f"  External needs count: {len(config.needs_external_needs)}")
    logger.info("=" * 80)


def setup_sphinx_extension(
    app: Any, needs_external_needs: List[Dict[str, Any]]
) -> Dict[str, Any]:
    """
    Setup function for Sphinx extension.

    This should be called from the conf.py setup() function to register
    the configuration verification callback.

    Args:
        app: Sphinx application instance
        needs_external_needs: List of external needs configurations

    Returns:
        Extension metadata dictionary
    """
    app.connect(
        "config-inited",
        lambda app, config: verify_config(app, config, needs_external_needs),
    )
    return {
        "version": "1.0",
        "parallel_read_safe": True,
        "parallel_write_safe": True,
    }
