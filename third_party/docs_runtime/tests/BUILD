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

load("@rules_shell//shell:sh_test.bzl", "sh_test")

# Smoke test: verify that the hermetic exec_in_sysroot graphviz wrapper can
# render a dot graph to SVG without a system graphviz installation.
sh_test(
    name = "dot_smoke_test",
    srcs = ["dot_smoke_test.sh"],
    data = [
        "//third_party/docs_runtime:dot",
    ],
    # The sysroot is Ubuntu 24.04 amd64; the dot binary is x86_64 ELF.
    target_compatible_with = [
        "@platforms//cpu:x86_64",
        "@platforms//os:linux",
    ],
)
