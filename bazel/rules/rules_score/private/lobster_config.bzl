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
Utilities for filling out Lobster traceability report config templates.

Report configs are static .tpl files in tools/lobster/templates/; the only
dynamic part is the list of source file paths for each level.  Rules call
format_lobster_sources() to build the source-line block, then pass it as a
substitution value to ctx.actions.expand_template().
"""

def format_lobster_sources(files):
    """Format a list of File objects as lobster source lines.

    Args:
        files: List of File objects to include as sources.

    Returns:
        String containing one ``  source: "...";`` line per file, suitable
        for substituting into a lobster config template placeholder.
    """
    return "\n".join(['  source: "{}";'.format(f.path) for f in files])
