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
        logger.debug(f"{NEEDS_EXTERNAL_FILE} not found - no external dependencies")
        return []

    logger.debug(f"Loading external needs from {NEEDS_EXTERNAL_FILE}")

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
    logger.debug(f"Workspace root: {workspace_root}")

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

        logger.debug(f"Added external needs config for '{key}':")
        logger.debug(f"  json_path: {config['json_path']}")
        logger.debug(f"  id_prefix: {config.get('id_prefix', 'none')}")
        logger.debug(f"  version: {config['version']}")

        external_needs.append(config)

    return external_needs


def init_external_needs(app: Any, config: Any) -> None:
    """
    Initialize external needs configuration.

    Args:
        app: Sphinx application object
        config: Sphinx configuration object
    """

    config.needs_external_needs = load_external_needs()


def setup(app: Any) -> Dict[str, Any]:
    """
    Sphinx setup hook to register event listeners.

    Args:
        app: Sphinx application object

    Returns:
        Extension metadata dictionary
    """
    app.connect("config-inited", init_external_needs)

    return {
        "version": "1.0",
        "parallel_read_safe": True,
        "parallel_write_safe": True,
    }
