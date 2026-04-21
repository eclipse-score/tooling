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
Aspect for collecting transitive dependencies from cc_library and cc_binary targets.

This module provides an aspect that traverses the build graph of C/C++ targets
and collects all labels that they depend on transitively.
"""

load("//bazel/rules/rules_score:providers.bzl", "CcDependencyInfo")

def _cc_dependencies_aspect_impl(target, ctx):
    collected_dependencies = depset([target.label])

    for attr_name in ["deps", "implementation_deps", "exported_deps"]:
        if hasattr(ctx.rule.attr, attr_name):
            deps = getattr(ctx.rule.attr, attr_name)
            for dep in deps:
                if CcDependencyInfo in dep:
                    collected_dependencies = depset(transitive = [collected_dependencies, dep[CcDependencyInfo].dependencies])

    return [CcDependencyInfo(
        dependencies = collected_dependencies,
    )]

cc_dependencies_aspect = aspect(
    implementation = _cc_dependencies_aspect_impl,
    attr_aspects = ["deps", "implementation_deps", "exported_deps"],
)
