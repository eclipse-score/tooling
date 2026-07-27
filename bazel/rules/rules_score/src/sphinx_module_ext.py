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
"""Sphinx extension entry point for loading external (cross-module) needs.

Registered by listing "sphinx_module_ext" in conf.py's `extensions = [...]`;
Sphinx then auto-invokes `setup(app)` below, no manual wiring required. This
is the counterpart to bazel_sphinx_needs.py, which offers the same
find_workspace_root()/load_external_needs() logic for conf.py authors who
prefer to import and wire it up explicitly instead of registering an
extension. See bazel_sphinx_needs.py's module docstring for that alternative.
"""

from typing import Any, Dict

from bazel_sphinx_needs import load_external_needs


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
