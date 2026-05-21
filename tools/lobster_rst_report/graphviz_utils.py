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

"""Shared Graphviz utilities for LOBSTER report tools."""

import subprocess
from typing import Optional


def is_dot_available(dot: Optional[str] = None) -> bool:
    """Return True if the ``dot`` executable (Graphviz) is on PATH.

    Args:
        dot: Optional explicit path to the ``dot`` binary.  When ``None``
            (default) the system PATH is searched.

    Returns:
        ``True`` if Graphviz ``dot`` is available, ``False`` otherwise.
    """
    try:
        subprocess.run(
            [dot if dot else "dot", "-V"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            encoding="UTF-8",
            check=True,
            timeout=5,
        )
        return True
    except (
        FileNotFoundError,
        subprocess.TimeoutExpired,
        subprocess.CalledProcessError,
    ):
        return False
