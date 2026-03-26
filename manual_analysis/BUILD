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

load("@manual_analysis_deps//:requirements.bzl", "requirement")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load(
    "//manual_analysis:manual_analysis_rules_test.bzl",
    "context_from_cc_library_provider_test",
    "context_from_filegroup_provider_test",
    "manual_analysis_macro_test_env_test",
    "manual_analysis_macro_update_env_test",
    "manual_analysis_rules_test_suite",
    "manual_analysis_test_env_test",
    "manual_analysis_update_env_test",
)

context_from_filegroup_provider_test(
    name = "context_from_filegroup_provider_test",
    target_under_test = "//manual_analysis/example:manual_analysis_context_from_filegroup",
)

context_from_cc_library_provider_test(
    name = "context_from_cc_library_provider_test",
    target_under_test = "//manual_analysis/example:manual_analysis_context_from_cc_library",
)

manual_analysis_update_env_test(
    name = "manual_analysis_macro_update_env_test",
    target_under_test = "//manual_analysis/example:manual_analysis.update",
)

manual_analysis_test_env_test(
    name = "manual_analysis_macro_test_env_test",
    target_under_test = "//manual_analysis/example:manual_analysis",
)

manual_analysis_rules_test_suite(
    name = "manual_analysis_rules_test_suite",
)

py_library(
    name = "common",
    srcs = ["common.py"],
    visibility = ["//visibility:public"],
)

py_library(
    name = "yaml_schema",
    srcs = ["yaml_schema.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [requirement("pyyaml")],
)

py_library(
    name = "interactive_runner_runtime_lib",
    srcs = ["interactive_runner_runtime.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_library(
    name = "interactive_runner_prefill_lib",
    srcs = ["interactive_runner_prefill.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_library(
    name = "interactive_runner_ui_console_lib",
    srcs = ["interactive_runner_ui_console.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [
        ":interactive_runner_runtime_lib",
        ":yaml_schema",
    ],
)

py_library(
    name = "interactive_runner_ui_split_lib",
    srcs = ["interactive_runner_ui_split.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [
        ":interactive_runner_runtime_lib",
        ":yaml_schema",
        requirement("prompt-toolkit"),
    ],
)

py_library(
    name = "interactive_runner_ui_lib",
    srcs = ["interactive_runner_ui.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [
        ":interactive_runner_runtime_lib",
        ":interactive_runner_ui_console_lib",
        ":interactive_runner_ui_split_lib",
    ],
)

py_library(
    name = "interactive_runner_steps_lib",
    srcs = ["interactive_runner_steps.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [
        ":interactive_runner_prefill_lib",
        ":interactive_runner_ui_console_lib",
        ":interactive_runner_ui_split_lib",
        ":yaml_schema",
    ],
)

py_library(
    name = "interactive_runner_flow_lib",
    srcs = ["interactive_runner_flow.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [
        ":interactive_runner_prefill_lib",
        ":interactive_runner_steps_lib",
        ":interactive_runner_ui_lib",
        ":yaml_schema",
    ],
)

py_library(
    name = "interactive_runner_cli_lib",
    srcs = ["interactive_runner_cli.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [
        ":common",
        ":interactive_runner_flow_lib",
        ":interactive_runner_prefill_lib",
        ":interactive_runner_runtime_lib",
        ":interactive_runner_steps_lib",
        ":yaml_schema",
    ],
)

py_library(
    name = "interactive_runner_lib",
    srcs = ["interactive_runner.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [
        ":interactive_runner_cli_lib",
        ":interactive_runner_flow_lib",
        ":interactive_runner_prefill_lib",
        ":interactive_runner_runtime_lib",
        ":interactive_runner_steps_lib",
        ":interactive_runner_ui_console_lib",
        ":interactive_runner_ui_lib",
        ":interactive_runner_ui_split_lib",
        ":yaml_schema",
    ],
)

py_binary(
    name = "interactive_runner",
    srcs = ["interactive_runner.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [":interactive_runner_lib"],
)

py_binary(
    name = "update_lock",
    srcs = ["update_lock.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [":update_lock_lib"],
)

py_library(
    name = "update_lock_lib",
    srcs = ["update_lock.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [":common"],
)

py_binary(
    name = "check_lock",
    srcs = ["check_lock.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [":check_lock_lib"],
)

py_library(
    name = "check_lock_lib",
    srcs = ["check_lock.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [":common"],
)

py_library(
    name = "check_results_lib",
    srcs = ["check_results.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [":common"],
)

py_binary(
    name = "check_results",
    srcs = ["check_results.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
    deps = [":check_results_lib"],
)

py_test(
    name = "yaml_schema_test",
    srcs = ["yaml_schema_test.py"],
    main = "yaml_schema_test.py",
    deps = [":yaml_schema"],
)

py_test(
    name = "check_results_test",
    srcs = ["check_results_test.py"],
    main = "check_results_test.py",
    deps = [":check_results_lib"],
)

py_test(
    name = "interactive_runner_ui_test",
    srcs = ["interactive_runner_ui_test.py"],
    main = "interactive_runner_ui_test.py",
    deps = [
        ":interactive_runner_ui_console_lib",
        ":interactive_runner_ui_split_lib",
        ":yaml_schema",
    ],
)

py_test(
    name = "interactive_runner_steps_test",
    srcs = ["interactive_runner_steps_test.py"],
    main = "interactive_runner_steps_test.py",
    deps = [
        ":interactive_runner_steps_lib",
        ":yaml_schema",
    ],
)

py_test(
    name = "interactive_runner_flow_cli_test",
    srcs = ["interactive_runner_flow_cli_test.py"],
    main = "interactive_runner_flow_cli_test.py",
    deps = [
        ":interactive_runner_cli_lib",
        ":interactive_runner_flow_lib",
        ":interactive_runner_prefill_lib",
        ":interactive_runner_runtime_lib",
        ":yaml_schema",
    ],
)

py_test(
    name = "interactive_runner_prefill_test",
    srcs = ["interactive_runner_prefill_test.py"],
    main = "interactive_runner_prefill_test.py",
    deps = [":interactive_runner_prefill_lib"],
)

py_test(
    name = "interactive_runner_runtime_test",
    srcs = ["interactive_runner_runtime_test.py"],
    main = "interactive_runner_runtime_test.py",
    deps = [":interactive_runner_runtime_lib"],
)

py_test(
    name = "interactive_runner_ui_factory_test",
    srcs = ["interactive_runner_ui_factory_test.py"],
    main = "interactive_runner_ui_factory_test.py",
    deps = [
        ":interactive_runner_ui_console_lib",
        ":interactive_runner_ui_lib",
        ":interactive_runner_ui_split_lib",
    ],
)

py_test(
    name = "interactive_runner_facade_test",
    srcs = ["interactive_runner_facade_test.py"],
    main = "interactive_runner_facade_test.py",
    deps = [
        ":interactive_runner_flow_lib",
        ":interactive_runner_lib",
        ":interactive_runner_prefill_lib",
    ],
)

py_test(
    name = "common_test",
    srcs = ["common_test.py"],
    main = "common_test.py",
    deps = [":common"],
)

py_test(
    name = "check_lock_test",
    srcs = ["check_lock_test.py"],
    main = "check_lock_test.py",
    deps = [":check_lock_lib"],
)

py_test(
    name = "update_lock_test",
    srcs = ["update_lock_test.py"],
    main = "update_lock_test.py",
    deps = [":update_lock_lib"],
)

py_test(
    name = "interactive_runner_ui_console_test",
    srcs = ["interactive_runner_ui_console_test.py"],
    main = "interactive_runner_ui_console_test.py",
    deps = [
        ":interactive_runner_ui_console_lib",
        ":yaml_schema",
    ],
)

# Run: bazel run //manual_analysis:requirements.update
compile_pip_requirements(
    name = "requirements",
    src = "requirements.txt.in",
    requirements_txt = "requirements.txt",
)
