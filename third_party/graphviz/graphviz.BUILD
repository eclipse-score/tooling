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
# graphviz_deb rule. It exposes the cmake-built graphviz binaries and their
# bundled shared libraries.
#
# == Use case ==
# We use graphviz exclusively to render the LOBSTER tracing-policy diagram as
# SVG inside Sphinx (sphinx.ext.graphviz, -Tsvg, dot layout algorithm).
#
# == Plugins activated for our use case ==
# Only two of the bundled plugins are activated at runtime for -Tsvg + dot layout:
#   libgvplugin_core.so.6      — SVG/PS/JSON renderer ("render: svg:core")
#   libgvplugin_dot_layout.so.6 — Hierarchical "dot" layout algorithm
#
# All other plugins in usr/lib/graphviz/ (pango, gd, neato_layout, vt, …) are
# registered at startup from the config6 file and then loaded on demand.
# For our -Tsvg + dot-layout use case they are never invoked; if their system
# dependencies are absent, graphviz emits a warning but SVG output is unaffected.
#
# == System library dependencies ==
# The cmake deb bundles all graphviz-specific .so files so that `dot_builtins`
# finds them via RUNPATH=$ORIGIN/../lib without a system graphviz installation.
# The remaining system libraries are split by whether they are required for our
# specific use case or only pulled in by unused plugins:
#
# Required by libgvplugin_core + libgvplugin_dot_layout (our actual use case):
#   libc.so.6     — C standard library (always present)
#   libm.so.6     — math library (always present)
#   libz.so.1     — zlib compression (always present: zlib1g)
#   libexpat.so.1 — XML/SVG parsing (always present: libexpat1)
#   libltdl.so.7  — plugin dynamic loader (libtool) (always present: libltdl7)
#
# Only required by unused plugins (pango/gd/neato_layout) — NOT needed for SVG:
#   libcairo.so.2 + libpixman-1.so.0 + libxcb*.so — raster/PDF rendering
#       (pango+gd plugins; pre-installed on Ubuntu 24.04)
#   libpango*.so + libfontconfig.so.1 + libfreetype.so.6
#       + libharfbuzz.so.0 + libfribidi.so.0 + libthai.so.0 + libdatrie.so.1
#       + libgraphite2.so.3 — font layout for PNG/PDF output
#       (pango plugin; pre-installed on Ubuntu 24.04)
#   libgd.so.3 + libjpeg.so.8 + libpng16.so.16 + libtiff.so.6 + libwebp.so.7
#       + libheif.so.1 + libLerc.so.4 + libjbig.so.0 + libdeflate.so.0
#       + libbrotli*.so + libzstd.so.1 + liblzma.so.5 + libsharpyuv.so.0
#       — image-format decoders for PNG/GIF/JPEG output
#       (gd plugin; pre-installed on Ubuntu 24.04)
#   libgts-0.7.so.5 — graph triangulation
#       (neato_layout only; NOT needed for dot layout)
#   libglib-2.0.so.0 + libgio-2.0.so.0 + libgobject-2.0.so.0
#       + libgmodule-2.0.so.0 + libffi.so.8 + libpcre2-8.so.0
#       + libblkid.so.1 + libmount.so.1 + libselinux.so.1
#       + libbsd.so.0 + libmd.so.0 — GLib/GIO stack (pango plugin transitive deps)
#       (pre-installed on Ubuntu 24.04)
#   libX11.so.6 + libXext.so.6 + libXrender.so.1 + libXpm.so.4
#       + libXau.so.6 + libXdmcp.so.6 — X11 display (xlib/x11 output only)
#       (pre-installed on Ubuntu 24.04)
#   libstdc++.so.6 + libgcc_s.so.1 — C++ runtime (gd plugin)
#       (always present)

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
