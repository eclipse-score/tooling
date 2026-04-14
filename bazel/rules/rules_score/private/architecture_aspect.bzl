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
Architecture Aspect for collecting component and unit structure.

This module provides an aspect that traverses the build graph to collect
architectural information about components and their contained units.
"""

load("//bazel/rules/rules_score:providers.bzl", "ComponentInfo", "UnitInfo")

CurrentArchitectureProviderInfo = provider(
    doc = "Provider for collecting component and unit architecture from the build graph",
    fields = {
        "components": "Dictionary mapping component labels to their structure (units and nested components)",
    },
)

def _collect_current_architecture_aspect_impl(target, ctx):
    components_dict = {}

    # Process components attribute - this contains both units and nested components
    if hasattr(ctx.rule.attr, "components"):
        current_units = []
        current_components = []

        for comp in ctx.rule.attr.components:
            comp_label = str(comp.label)

            # Check if it's a unit or a component
            if UnitInfo in comp:
                current_units.append(comp_label)
            elif ComponentInfo in comp:
                current_components.append(comp_label)

            if CurrentArchitectureProviderInfo in comp:
                nested_info = comp[CurrentArchitectureProviderInfo]

                # Merge all nested components into flat hierarchy
                components_dict.update(nested_info.components)

        component_structure = {}
        if current_units:
            component_structure["units"] = current_units
        if current_components:
            component_structure["components"] = current_components
        components_dict[str(target.label)] = component_structure

    return [CurrentArchitectureProviderInfo(
        components = components_dict,
    )]

collect_current_architecture_aspect = aspect(
    implementation = _collect_current_architecture_aspect_impl,
    attr_aspects = ["components"],
)
