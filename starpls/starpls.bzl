# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
load("@rules_shell//shell:sh_binary.bzl", "sh_binary")

def setup_starpls(name, visibility = ["//visibility:public"]):
    native.genrule(
        name = name,
        outs = [name + "_bin"],
        cmd = """
            echo "Downloading starpls binary via genrule defined in macro..." >&2
            curl -fsSL "https://github.com/withered-magic/starpls/releases/download/v0.1.21/starpls-linux-amd64" -o "$@" && \
            chmod +x "$@"
            echo "Download complete: $@" >&2
        """,
        executable = True,
    )
