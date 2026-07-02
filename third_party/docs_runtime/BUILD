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

# Hermetic doc-tool runtime rootfs. The @docs_runtime repo (rules_distroless
# apt.install) is defined in //MODULE.bazel; its manifest + lock live here.

load("//bazel/rules/exec_in_sysroot:exec_in_sysroot.bzl", "exec_in_sysroot", "prepare_sysroot")

package(default_visibility = ["//visibility:public"])

exports_files([
    "docs_runtime.yaml",
    "docs_runtime.lock.json",
])

# Rework the distroless rootfs once into a cached `dot`-ready sysroot archive.
# The apt graphviz package ships /usr/bin/dot as a symlink to
# libgvc6-config-update; its postinst (`dot -c`) generates config6a so dot can
# locate and load its plugins at runtime.  rules_distroless never runs postinst,
# so we generate it here while the sysroot is still writable.  Keeping this in a
# separate rule lets Bazel cache the result (the expensive `dot -c` step is not
# re-run when only the wrapped executable / exclude_paths change).
prepare_sysroot(
    name = "dot_sysroot",
    exec_compatible_with = [
        "@platforms//cpu:x86_64",
        "@platforms//os:linux",
    ],
    # Prune plugins whose X11/pango/GD dependencies are absent from this minimal
    # sysroot (they cannot be dlopened, causing dot -c to crash).  Use find (not
    # a hardcoded arch path) so the rule works on both amd64 and aarch64.
    host_setup_commands = [
        "find \"$SYSROOT/usr/lib\" -name 'libgvplugin_pango*' -delete",
        "find \"$SYSROOT/usr/lib\" -name 'libgvplugin_gd*' -delete",
        "find \"$SYSROOT/usr/lib\" -name 'libgvplugin_xlib*' -delete",
        "find \"$SYSROOT/usr/lib\" -name 'libgvplugin_webp*' -delete",
        "find \"$SYSROOT/usr/lib\" -name 'libgvplugin_visio*' -delete",
    ],
    sysroot = "@docs_runtime//:flat",
    sysroot_setup_commands = [
        # Generate config6a for the remaining plugins.  Runs inside the sysroot
        # context so FAKECHROOT_BASE redirects /usr/bin/dot to the sysroot's dot
        # and the resulting config6a is written into the sysroot.
        "/usr/bin/dot -c",
    ],
)

# Hermetic graphviz dot executable used by doc actions.
exec_in_sysroot(
    name = "dot",
    # /tmp must not be chrooted so that PlantUML's per-diagram .dot / .svg temp
    # files (written to the real /tmp) are accessible to dot without being
    # redirected into the sysroot.  Graphviz-internal paths (/usr/lib/…/graphviz,
    # /usr/share/fonts, …) are still correctly redirected by fakechroot.
    exclude_paths = ["/tmp"],
    exec_compatible_with = [
        "@platforms//cpu:x86_64",
        "@platforms//os:linux",
    ],
    sysroot = ":dot_sysroot",
    sysroot_binary = "/usr/bin/dot",
)
