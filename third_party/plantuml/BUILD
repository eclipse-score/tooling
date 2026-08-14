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

load("@rules_java//java:defs.bzl", "java_binary")
load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load("//bazel/rules/exec_in_sysroot:exec_in_sysroot.bzl", "extract_sysroot")

extract_sysroot(
    name = "plantuml_sysroot",
    sysroot = "//third_party/docs_runtime:dot_sysroot",
)

java_binary(
    name = "plantuml_java",
    main_class = "net.sourceforge.plantuml.Run",
    runtime_deps = [
        "@blueprint_maven_dependencies//:net_sourceforge_plantuml_plantuml",
    ],
)

sh_binary(
    name = "plantuml",
    srcs = ["plantuml.sh"],
    data = [
        ":plantuml_java",
        ":plantuml_sysroot",
    ],
    visibility = ["//visibility:public"],
)
