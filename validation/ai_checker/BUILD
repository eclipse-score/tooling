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
    srcs = glob(["guidelines/*.md"]),
    visibility = ["//visibility:public"],
)

# Core AI checker library (analysis framework)
py_library(
    name = "ai_checker_core",
    srcs = glob(["src/ai_checker/*.py"]),
    imports = ["src"],
    visibility = ["//visibility:public"],
    deps = [
        "@trlc//trlc",
        requirement("bigtree"),
        requirement("pydantic"),
        requirement("pydot"),
        requirement("pyyaml"),
    ],
)

# LangChain adapter for GitHub Copilot SDK
py_library(
    name = "copilot_langchain",
    srcs = [
        "src/copilot_adapter/__init__.py",
        "src/copilot_adapter/_client_manager.py",
        "src/copilot_adapter/_errors.py",
        "src/copilot_adapter/_message_converter.py",
        "src/copilot_adapter/_preflight.py",
        "src/copilot_adapter/_tool_converter.py",
        "src/copilot_adapter/copilot_langchain.py",
    ],
    imports = ["src"],
    visibility = ["//visibility:public"],
    deps = [
        requirement("langchain-core"),
        requirement("github-copilot-sdk"),
        requirement("pydantic"),
    ],
)

# Default orchestrator (uses GitHub Copilot SDK as default AI backend)
py_binary(
    name = "orchestrator",
    srcs = ["src/ai_checker/orchestrator.py"],
    imports = ["src"],
    main = "src/ai_checker/orchestrator.py",
    visibility = ["//visibility:public"],
    deps = [
        ":ai_checker_core",
        ":copilot_langchain",
        requirement("langchain-core"),
    ],
)

# Run: bazel run //validation/ai_checker:requirements.update
compile_pip_requirements(
    name = "requirements",
    src = "requirements.txt.in",
    requirements_txt = "requirements.txt",
)
