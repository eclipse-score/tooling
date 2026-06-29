#!/bin/sh
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

set -eu

# Use the sysroot's own ELF interpreter (exported by exec_in_sysroot.sh as
# SYSROOT_INTERP) with an explicit --library-path so that dot and all of its
# shared-library dependencies (libgvc.so.6, libcgraph.so.6, …) are loaded
# entirely from the sysroot.
#
# Running dot through the HOST's ld-linux.so and a wide LD_LIBRARY_PATH would
# cause the sysroot's libc.so.6 to load alongside the host's already-resident
# libc (two libc instances → segfault on systems without a host graphviz).
# The sysroot's own interpreter gives a single coherent libc.
#
# LD_PRELOAD=libfakechroot.so + FAKECHROOT_BASE are still active (set by
# exec_in_sysroot.sh) so glibc-level filesystem calls inside dot (e.g.
# opening the graphviz plugin directory, reading config6) are transparently
# redirected into the sysroot.
exec "${SYSROOT_INTERP}" --library-path "${SYSROOT_LIBPATH}" "${SYSROOT_DIR}/usr/bin/dot" "$@"
