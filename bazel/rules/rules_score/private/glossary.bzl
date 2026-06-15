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
Glossary build rule for S-CORE projects.

This rule accepts glossary sources as ``.rst`` files and publishes them
through ``SphinxSourcesInfo`` for the ``dependable_element`` documentation
assembly pipeline.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _glossary_impl(ctx):
    """Implementation for ``glossary`` rule.

    Args:
        ctx: Rule context.

    Returns:
        List of providers including:
        - ``DefaultInfo`` with glossary source files
        - ``SphinxSourcesInfo`` where ``srcs == deps`` (leaf rule)
    """

    source_files = depset(ctx.files.srcs)

    return [
        DefaultInfo(files = source_files),
        SphinxSourcesInfo(
            srcs = source_files,
            deps = source_files,
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_glossary = rule(
    implementation = _glossary_impl,
    doc = "Collect glossary documentation files and expose them to Sphinx.",
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".rst"],
            mandatory = True,
            doc = "Glossary source files (``.rst``).",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def glossary(name, srcs, **kwargs):
    """Define a glossary artifact for documentation assembly.

    Args:
        name: Target name.
        srcs: List of glossary source files (.rst).
        visibility: Bazel visibility specification.
    """

    _glossary(
        name = name,
        srcs = srcs,
        **kwargs
    )
