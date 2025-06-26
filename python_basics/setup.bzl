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

"""
Setup utilities for score_python_basics that simplify Python configuration.

This module provides convenient functions and documentation to help consumers
set up their Python dependencies with minimal boilerplate.
"""

# Python version used by score_python_basics (matches MODULE.bazel)
PYTHON_VERSION = "3.12"
RULES_PYTHON_VERSION = "1.4.1"

def python_deps_extension():
    """
    Returns configuration information for score_python_basics consumers.
    
    This function provides the minimal MODULE.bazel configuration patterns
    and version information for using score_python_basics effectively.
    
    Returns:
        dict: Configuration information including Python version and patterns
    """
    return {
        "python_version": PYTHON_VERSION,
        "rules_python_version": RULES_PYTHON_VERSION,
        "patterns": {
            "no_pip_deps": """bazel_dep(name = "score_python_basics", version = "0.3.0")""",
            "with_pip_deps": """bazel_dep(name = "score_python_basics", version = "0.3.0")

pip = use_extension("@rules_python//python/extensions:pip.bzl", "pip")
pip.parse(
    hub_name = "pip",
    python_version = "{python_version}",
    requirements_lock = "//path/to:requirements.txt",
)
use_repo(pip, "pip")""".format(python_version=PYTHON_VERSION),
        }
    }

def validate_pip_config(python_version, requirements_lock=None):
    """
    Validates pip configuration for compatibility with score_python_basics.
    
    Args:
        python_version (str): Python version specified in pip.parse()
        requirements_lock (str, optional): Path to requirements file
        
    Returns:
        dict: Validation result with 'valid' boolean and 'messages' list
    """
    messages = []
    valid = True
    
    if python_version != PYTHON_VERSION:
        valid = False
        messages.append(
            f"Python version mismatch: score_python_basics uses {PYTHON_VERSION}, "
            f"but you specified {python_version}. "
            f"Please change to python_version = \"{PYTHON_VERSION}\""
        )
    
    if requirements_lock and not requirements_lock.endswith(('.txt', '.lock')):
        messages.append(
            f"Warning: requirements file '{requirements_lock}' should typically "
            f"end with .txt or .lock"
        )
    
    if valid:
        messages.append("✅ Configuration is compatible with score_python_basics")
    
    return {
        "valid": valid,
        "messages": messages
    }

# For backwards compatibility and convenience, export the version info
SCORE_PYTHON_VERSION = PYTHON_VERSION