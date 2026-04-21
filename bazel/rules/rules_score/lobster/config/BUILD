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

# Lobster-trlc YAML configs and report config templates for S-CORE traceability.
# Used as defaults by rules_score rules (component, dependable_element, fmea, etc.).

filegroup(
    name = "component_requirement",
    srcs = ["lobster_comp_req.yaml"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "feature_requirement",
    srcs = ["lobster_feat_req.yaml"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "failuremodes_config",
    srcs = ["lobster_failuremodes.yaml"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "controlmeasures_config",
    srcs = ["lobster_controlmeasures.yaml"],
    visibility = ["//visibility:public"],
)

# ---------------------------------------------------------------------------
# Lobster report config templates: static structure with {PLACEHOLDER} markers
# for source file lists. Used by rules_score to generate per-target configs
# via ctx.actions.expand_template() — only the source paths vary.
# ---------------------------------------------------------------------------
filegroup(
    name = "lobster_component_template",
    srcs = ["lobster_component.conf.tpl"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "lobster_de_template",
    srcs = ["lobster_de.conf.tpl"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "lobster_sa_template",
    srcs = ["lobster_sa.conf.tpl"],
    visibility = ["//visibility:public"],
)
