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
load("@pip_ai_checker//:requirements.bzl", "requirement")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")

filegroup(
    name = "readme_md",
    srcs = ["README.md"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "doc_assets",
    srcs = glob(["_assets/*.svg"]),
    visibility = ["//visibility:public"],
)

# Default requirements engineering guidelines
filegroup(
    name = "default_guidelines",
    srcs = ["guidelines/general.md"] + glob(["guidelines/requirements/*.md"]),
    visibility = ["//visibility:public"],
)

# Default architecture design guidelines
filegroup(
    name = "default_architecture_guidelines",
    srcs = ["guidelines/general.md"] + glob(["guidelines/architecture/*.md"]),
    visibility = ["//visibility:public"],
)

# Empty default for the project-guideline flags below. Consumer repos override
# the flags in their .bazelrc to inject project-specific guidelines once,
# instead of setting an attribute on every AI test target.
filegroup(
    name = "empty_guidelines",
    srcs = [],
    visibility = ["//visibility:public"],
)

# Project-specific requirement guidelines (graded). Override once via:
#   build --//validation/ai_checker:project_guidelines=//path/to:my_guidelines
label_flag(
    name = "project_guidelines",
    build_setting_default = ":empty_guidelines",
    visibility = ["//visibility:public"],
)

# Project-specific architecture guidelines (graded). Override once via:
#   build --//validation/ai_checker:project_architecture_guidelines=//path/to:my_guidelines
label_flag(
    name = "project_architecture_guidelines",
    build_setting_default = ":empty_guidelines",
    visibility = ["//visibility:public"],
)

# Example SCORE project guidelines, referenced by the flags above when a repo
# opts into the SCORE process levels.
filegroup(
    name = "score_project_guidelines",
    srcs = ["guidelines/project/score_requirement_levels.md"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "score_project_architecture_guidelines",
    srcs = ["guidelines/project/score_architecture_levels.md"],
    visibility = ["//visibility:public"],
)

# Core AI checker library (analysis framework).
# Deliberately free of any AI-SDK / LangChain dependency: it depends only on
# the AnalysisAgent interface, which concrete agents implement.
# Core analysis framework + extractors + the AnalysisAgent interface.
# No AI-SDK / LangChain dependency. The agents/ subpackage (concrete backends)
# is intentionally excluded — those are separate libraries below.
py_library(
    name = "ai_checker_core",
    srcs = glob([
        "src/ai_checker/*.py",
        "src/ai_checker/extractors/*.py",
        "src/ai_checker/reports/*.py",
    ]),
    # Jinja2 report templates are loaded at runtime relative to the package,
    # so they must travel in the library's runfiles.
    data = glob(["src/ai_checker/reports/*.j2"]),
    imports = ["src"],
    visibility = ["//visibility:public"],
    deps = [
        "@trlc//trlc",
        requirement("bigtree"),
        requirement("jinja2"),
        requirement("markupsafe"),
        requirement("pydantic"),
        requirement("pydot"),
        requirement("pyyaml"),
    ],
)

# Default AI backend: GitHub Copilot SDK agent (no LangChain).
py_library(
    name = "copilot_agent",
    srcs = [
        "src/ai_checker/agents/__init__.py",
        "src/ai_checker/agents/_client_manager.py",
        "src/ai_checker/agents/_errors.py",
        "src/ai_checker/agents/_preflight.py",
        "src/ai_checker/agents/copilot_agent.py",
    ],
    imports = ["src"],
    visibility = ["//visibility:public"],
    deps = [
        ":ai_checker_core",
        requirement("github-copilot-sdk"),
        requirement("pydantic"),
    ],
)

# Optional LangChain adapter: wraps any LangChain BaseChatModel (e.g. ChatOpenAI)
# as an AnalysisAgent. Used only when a custom langchain model is supplied via a
# custom ai_model target's create_agent().
py_library(
    name = "langchain_agent",
    srcs = [
        "src/ai_checker/agents/__init__.py",
        "src/ai_checker/agents/_errors.py",
        "src/ai_checker/agents/langchain_agent.py",
    ],
    imports = ["src"],
    visibility = ["//visibility:public"],
    deps = [
        ":ai_checker_core",
        requirement("langchain-core"),
    ],
)

# Default orchestrator (uses the Copilot SDK agent as default AI backend).
# Intentionally does NOT depend on :langchain_agent — the default path is
# LangChain-free. A consumer wanting the LangChain path supplies a custom
# ai_model target whose create_agent() returns a LangChainAgent and which
# depends on :langchain_agent itself.
py_binary(
    name = "orchestrator",
    srcs = ["src/ai_checker/orchestrator.py"],
    imports = ["src"],
    main = "src/ai_checker/orchestrator.py",
    visibility = ["//visibility:public"],
    deps = [
        ":ai_checker_core",
        ":copilot_agent",
    ],
)

# In order to update the requirements, change the `requirements.txt` file and run:
# `bazel run //:requirements_3_XX.update  --@@rules_python+//python/config_settings:python_version=3.XX`.
# This will update the `requirements_lock.txt` file.
# To upgrade all dependencies to their latest versions, run:
# `bazel run //:requirements_3_XX.update  --@@rules_python+//python/config_settings:python_version=3.XX -- --upgrade`.
[
    compile_pip_requirements(
        name = "requirements_3_{}".format(version),
        src = "requirements.txt.in",
        python_version = "3.{}".format(version),
        requirements_txt = "requirements_3_{}.txt".format(version),
        tags = [
            "manual",
        ],
    )
    for version in [
        "11",
        "12",
    ]
]
