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

# This BUILD file is injected into the @graphviz_deb external repository by the
# graphviz_deb rule. It exposes dot_builtins and required bundled shared
# libraries for Sphinx graphviz rendering in a hermetic way.

package(default_visibility = ["//visibility:public"])

# The actual graphviz rendering binary (not the dot wrapper/launcher).
# Uses RUNPATH $ORIGIN/../lib to find bundled shared libraries.
filegroup(
    name = "dot_binary",
    srcs = ["usr/bin/dot_builtins"],
)

# Bundled graphviz shared libraries (libgvc, libcgraph, libcdt, libpathplan, libxdot).
# These are found automatically by dot_builtins via RUNPATH $ORIGIN/../lib.
filegroup(
    name = "core_libs",
    srcs = glob(["usr/lib/*.so*"]),
)

# Graphviz plugin shared libraries (libgvplugin_core, libgvplugin_dot_layout, etc.).
# Loaded at runtime via libltdl; requires LTDL_LIBRARY_PATH=usr/lib/graphviz.
filegroup(
    name = "plugin_libs",
    srcs = glob(["usr/lib/graphviz/*.so*"]),
)

# All graphviz files needed to run dot_builtins.
filegroup(
    name = "all",
    srcs = [
        ":core_libs",
        ":dot_binary",
        ":plugin_libs",
    ],
)
