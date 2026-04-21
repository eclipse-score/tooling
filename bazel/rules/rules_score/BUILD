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
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load(
    "//bazel/rules/rules_score:rules_score.bzl",
    "sphinx_module",
)
load("//bazel/rules/rules_score:sphinx_toolchain.bzl", "sphinx_toolchain")
load("//lobster_bazel:lobster_bazel.bzl", "lobster_linker")

exports_files([
    "templates/conf.template.py",
    "templates/dependable_element_index.template.rst",
    "templates/unit.template.rst",
    "templates/component.template.rst",
    "templates/fmea.template.rst",
])

compile_pip_requirements(
    name = "requirements",
    src = "requirements.in",
    requirements_txt = "requirements.txt",
    tags = [
        "manual",
    ],
)

# Arch-to-reqs-from-lobster tool: extracts requirements from component requirement .lobster files
# and generates an architecture.lobster item representing the component's allocation
py_binary(
    name = "arch_to_reqs_from_lobster",
    srcs = ["src/arch_to_reqs_from_lobster.py"],
    main = "src/arch_to_reqs_from_lobster.py",
    visibility = ["//visibility:public"],
)

# Safety analysis tools (preprocess FTA PlantUML + extract lobster traceability items)
py_binary(
    name = "safety_analysis_tools",
    srcs = ["src/safety_analysis_tools.py"],
    imports = ["src"],
    main = "src/safety_analysis_tools.py",
    visibility = ["//visibility:public"],
)

# HTML merge tool
py_binary(
    name = "sphinx_html_merge",
    srcs = ["src/sphinx_html_merge.py"],
    main = "src/sphinx_html_merge.py",
    visibility = ["//visibility:public"],
)

py_library(
    name = "sphinx_module_ext",
    srcs = ["src/sphinx_module_ext.py"],
    imports = ["src"],
    deps = [
        requirement("sphinx"),
    ],
)

sphinx_module(
    name = "rules_score_doc",
    srcs = glob(
        [
            "docs/**/*.rst",
            "docs/**/*.puml",
        ],
        allow_empty = True,
    ),
    index = "docs/index.rst",
    visibility = ["//visibility:public"],
)

# HTML merge tool

py_binary(
    name = "raw_build",
    srcs = ["src/sphinx_wrapper.py"],
    data = [
        "//tools/sphinx:plantuml",
    ],
    env = {
        "SOURCE_DIRECTORY": "",
        "DATA": "",
        "ACTION": "check",
    },
    main = "src/sphinx_wrapper.py",
    visibility = ["//visibility:public"],
    deps = [
        ":sphinx_module_ext",
        "@lobster//sphinx_lobster:sphinx_lobster_builder",
        "@rules_python//python/runfiles",
        "@score_tooling//plantuml/sphinx/clickable_plantuml",
        "@trlc//tools/sphinx/extensions/trlc",
        requirement("sphinx"),
        requirement("sphinx_rtd_theme"),
        requirement("sphinx_needs"),
        requirement("sphinx_design"),
        requirement("myst_parser"),
        requirement("readthedocs_sphinx_ext"),
        requirement("rst2pdf"),
        requirement("sphinxcontrib-umlet"),
        requirement("svglib"),
        requirement("sphinxcontrib-plantuml"),
    ],
)

sphinx_toolchain(
    name = "default_toolchain",
    sphinx = ":raw_build",
)

toolchain_type(name = "toolchain_type")

toolchain(
    name = "sphinx_default_toolchain",
    toolchain = ":default_toolchain",
    toolchain_type = ":toolchain_type",
)

# ---------------------------------------------------------------------------
# Lobster traceability – rules_score Bazel implementation
# ---------------------------------------------------------------------------

filegroup(
    name = "bzl_srcs",
    srcs = glob(["*.bzl"]),
)

lobster_linker(
    name = "rules_score_impl",
    srcs = [
        ":bzl_srcs",
        "//bazel/rules/rules_score/private:bzl_srcs",
    ],
    visibility = ["//docs/processes:__pkg__"],
)
