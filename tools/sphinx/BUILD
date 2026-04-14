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

load("@pip_rules_score//:requirements.bzl", "requirement")
load("@rules_java//java:defs.bzl", "java_binary")
load("@rules_python//sphinxdocs:sphinx.bzl", "sphinx_build_binary")
load("@pip_tooling//:requirements.bzl", "requirement")

java_binary(
    name = "plantuml",
    main_class = "net.sourceforge.plantuml.Run",
    visibility = ["//visibility:public"],
    runtime_deps = [
        "@blueprint_maven_dependencies//:net_sourceforge_plantuml_plantuml",
    ],
)

sphinx_build_binary(
    name = "sphinx-build",
    visibility = ["//visibility:public"],
    deps = [
        "@lobster//sphinx_lobster:sphinx_lobster_builder",
        "@score_tooling//plantuml/sphinx/clickable_plantuml",
        "@trlc//tools/sphinx/extensions/trlc",
        requirement("sphinx"),
        requirement("sphinx_rtd_theme"),
        requirement("myst_parser"),
        requirement("readthedocs_sphinx_ext"),
        requirement("rst2pdf"),
        requirement("sphinxcontrib-umlet"),
        requirement("svglib"),
        requirement("sphinxcontrib-plantuml"),
        "@rules_python//python/runfiles",
    ],
)
