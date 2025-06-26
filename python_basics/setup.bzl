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
Setup utilities for score_python_basics that simplify Python configuration.

This module provides convenient functions and documentation to help consumers
set up their Python dependencies with minimal boilerplate.
"""

# Python version used by score_python_basics (matches MODULE.bazel)
PYTHON_VERSION = "3.12"

def python_deps_extension():
    """
    Returns the minimal MODULE.bazel configuration for using score_python_basics
    with pip dependencies.
    
    This is a documentation function that shows the recommended pattern.
    Instead of the traditional ~20 lines of boilerplate, consumers only need:
    
    For projects without pip dependencies:
    ```starlark
    bazel_dep(name = "score_python_basics", version = "0.3.0")
    ```
    
    For projects with pip dependencies:
    ```starlark
    bazel_dep(name = "score_python_basics", version = "0.3.0")
    
    # Since score_python_basics already includes rules_python and configures 
    # Python 3.12 toolchain, you only need to configure your pip dependencies:
    pip = use_extension("@rules_python//python/extensions:pip.bzl", "pip")
    pip.parse(
        hub_name = "pip",
        python_version = "3.12",  # Must match score_python_basics version
        requirements_lock = "//path/to:requirements.txt",
    )
    use_repo(pip, "pip")
    ```
    
    This replaces the old boilerplate:
    ```starlark
    # ❌ No longer needed - score_python_basics handles this:
    # bazel_dep(name = "rules_python", version = "1.4.1") 
    # python = use_extension("@rules_python//python/extensions:python.bzl", "python")
    # python.toolchain(is_default = True, python_version = "3.12")
    # use_repo(python)
    ```
    """
    return {
        "python_version": PYTHON_VERSION,
        "rules_python_version": "1.4.1",  # Version used by score_python_basics
    }

# For backwards compatibility and convenience, export the version info
SCORE_PYTHON_VERSION = PYTHON_VERSION
RULES_PYTHON_VERSION = "1.4.1"