# *******************************************************************************
# Copyright (c) 2024 Contributors to the Eclipse Foundation
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

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def score_cr_checker_deps():
    if not native.existing_rule("rules_cc"):
        http_archive(
            name = "rules_cc",
            urls = ["https://github.com/bazelbuild/rules_cc/releases/download/0.1.1/rules_cc-0.1.1.tar.gz"],
            sha256 = "712d77868b3152dd618c4d64faaddefcc5965f90f5de6e6dd1d5ddcd0be82d42",
            strip_prefix = "rules_cc-0.1.1",
        )
    if not native.existing_rule("aspect_rules_py"):
        http_archive(
            name = "rules_python",
            sha256 = "9f9f3b300a9264e4c77999312ce663be5dee9a56e361a1f6fe7ec60e1beef9a3",
            strip_prefix = "rules_python-1.4.1",
            url = "https://github.com/bazel-contrib/rules_python/releases/download/1.4.1/rules_python-1.4.1.tar.gz",
        )
    if not native.existing_rule("aspect_rules_py"):
        http_archive(
            name = "aspect_rules_py",
            sha256 = "8e9a1f00e4ba5696f9e93a770a6c1de863544cce489df91809fc3a4027ccfddc",
            strip_prefix = "rules_py-1.4.0",
            url = "https://github.com/aspect-build/rules_py/releases/download/v1.4.0/rules_py-v1.4.0.tar.gz",
        )
