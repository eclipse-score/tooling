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

load("@bazel_skylib//rules:common_settings.bzl", "string_flag")
load("@pip_rules_score//:requirements.bzl", "requirement")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@rules_python//sphinxdocs:sphinx_docs_library.bzl", "sphinx_docs_library")
load(
    "//bazel/rules/rules_score:rules_score.bzl",
    "sphinx_module",
)
load("//bazel/rules/rules_score:sphinx_toolchain.bzl", "sphinx_toolchain")
load("//lobster_bazel:lobster_bazel.bzl", "lobster_linker")

exports_files([
    "src/bazel_sphinx_needs.py",
    "src/sphinx_wrapper.py",
    "templates/conf.template.py",
    "templates/dependable_element_index.template.rst",
    "templates/unit.template.rst",
    "templates/component.template.rst",
    "templates/fmea.template.rst",
    "templates/puml_diagram.template.rst",
])

compile_pip_requirements(
    name = "requirements",
    src = "requirements.in",
    requirements_txt = "requirements.txt",
    tags = [
        "manual",
    ],
)

# RST-to-TRLC converter: converts RST requirement directives to TRLC format
py_library(
    name = "rst_to_trlc_lib",
    srcs = ["src/rst_to_trlc.py"],
    imports = ["src"],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "rst_to_trlc",
    srcs = ["src/rst_to_trlc.py"],
    imports = ["src"],
    main = "src/rst_to_trlc.py",
    visibility = ["//visibility:public"],
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

# AoU forwarding filter: filters received AoU lobster entries for chain-forwarding
py_binary(
    name = "aou_forwarding_to_lobster",
    srcs = ["src/aou_forwarding_to_lobster.py"],
    imports = ["src"],
    main = "src/aou_forwarding_to_lobster.py",
    visibility = ["//visibility:public"],
    deps = [
        requirement("pyyaml"),
    ],
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
    visibility = ["//visibility:public"],
    deps = [
        requirement("sphinx"),
    ],
)

py_library(
    name = "bazel_sphinx_needs",
    srcs = ["src/bazel_sphinx_needs.py"],
    imports = ["src"],
    visibility = ["//visibility:public"],
    deps = [
        requirement("sphinx"),
    ],
)

# ---------------------------------------------------------------------------
# Tool reference pages: each README.md is renamed to <name>.md via
# renamed_srcs on sphinx_module. No sphinx_docs_library needed for renames —
# sphinx_docs_library only strips/prepends directory segments and cannot rename
# basenames.
# ---------------------------------------------------------------------------

# Assets referenced by ai_checker/README.md (relative ![](_assets/*.svg) links).
# These need directory-prefix stripping, not renaming → sphinx_docs_library is correct here.
sphinx_docs_library(
    name = "tool_readme_ai_checker_assets",
    srcs = ["//validation/ai_checker:doc_assets"],
    prefix = "bazel/rules/rules_score/docs/tool_reference/",
    strip_prefix = "validation/ai_checker/",
)

# Validation specifications — two files, same source dir, no rename needed.
sphinx_docs_library(
    name = "validation_specs",
    srcs = ["//validation/core:specifications"],
    prefix = "bazel/rules/rules_score/docs/tool_reference/specs/",
    strip_prefix = "validation/core/docs/specifications/",
)

sphinx_module(
    name = "rules_score_doc",
    srcs = glob(
        [
            "docs/**/*.rst",
            "docs/**/*.puml",
            "docs/**/*.md",
        ],
        allow_empty = True,
    ) + [
        "//bazel/rules/rules_score/docs/requirements:potential_errors_rst",
        "//bazel/rules/rules_score/docs/requirements:tool_requirements_rst",
        "//bazel/rules/rules_score/docs/requirements:traceability_rst",
        "//bazel/rules/rules_score/docs/requirements:use_cases_rst",
    ],
    docs_library_deps = [
        ":tool_readme_ai_checker_assets",
        ":validation_specs",
    ],
    index = "docs/index.rst",
    renamed_srcs = {
        "//plantuml/parser:readme_md": "bazel/rules/rules_score/docs/tool_reference/plantuml_parser.md",
        "//validation/core:README.md": "bazel/rules/rules_score/docs/tool_reference/validation_core.md",
        "//validation/ai_checker:readme_md": "bazel/rules/rules_score/docs/tool_reference/ai_checker.md",
        "//plantuml/sphinx/clickable_plantuml:readme_md": "bazel/rules/rules_score/docs/tool_reference/clickable_plantuml.md",
        "//lobster_bazel:readme_md": "bazel/rules/rules_score/docs/tool_reference/lobster_bazel.md",
        "//manual_analysis:readme_md": "bazel/rules/rules_score/docs/tool_reference/manual_analysis.md",
        "//validation/core/docs/requirements:requirements_rst": "bazel/rules/rules_score/docs/requirements/validation_core_requirements_rst.rst",
    },
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

toolchain_type(
    name = "toolchain_type",
    visibility = ["//visibility:public"],
)

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

filegroup(
    name = "py_srcs",
    srcs = glob(["src/*.py"]),
)

lobster_linker(
    name = "rules_score_impl",
    srcs = [
        ":bzl_srcs",
        ":py_srcs",
        "//bazel/rules/rules_score/private:bzl_srcs",
    ],
    visibility = [
        "//bazel/rules/rules_score/docs/requirements:__pkg__",
        "//docs/processes:__pkg__",
    ],
)

# ---------------------------------------------------------------------------
# Build setting: verbosity level for all rules_score tools
# ---------------------------------------------------------------------------

string_flag(
    name = "verbosity",
    build_setting_default = "warn",
    values = [
        "warn",
        "info",
        "debug",
    ],
    visibility = ["//visibility:public"],
)
