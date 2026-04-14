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
load(
    "//bazel/rules/rules_score:rules_score.bzl",
    "unit_design",
)

# Class diagrams are passed through directly to Sphinx (sphinxcontrib-plantuml
# renders them at doc-build time). Both the RST wrapper and the referenced
# .puml file must be listed together so they are symlinked into the same output
# directory and the '.. uml::' directive can find the diagram.
unit_design(
    name = "unit_design",
    static = [
        "unit_2_design.rst",
        "unit_2_class_diagram.puml",
    ],
    visibility = ["//visibility:public"],
)
