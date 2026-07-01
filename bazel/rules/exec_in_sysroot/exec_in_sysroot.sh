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
# Generic wrapper script that executes a command in fakechroot using the provided sysroot.
#
# Rather than using `fakechroot -- chroot SYSROOT COMMAND` (which requires COMMAND
# to exist inside the chroot), this script uses fakechroot's LD_PRELOAD mechanism
# directly: libfakechroot.so intercepts all file-system calls and redirects absolute
# paths to the sysroot, while FAKECHROOT_EXCLUDE_PATH allows the host-side launcher
# script to be found at its real location.
#
# Usage:
#   exec_in_sysroot.sh <sysroot-binary-path> [arguments...]
#
# Environment variables (set by the generated exec_in_sysroot wrapper):
#   SYSROOT_DIR             - Path to the extracted sysroot directory (required)
#   FAKECHROOT_EXCLUDE_PATH - Colon-separated paths NOT to redirect (optional)
#   BUILD_WORKSPACE_DIRECTORY - Workspace root, automatically excluded (optional)

set -eu

COMMAND_IN_SYSROOT="${1:?Command must be provided as first argument}"
shift

SYSROOT_DIR="${SYSROOT_DIR:?SYSROOT_DIR must be set}"

FAKECHROOT_LIB=""
if [ -f "${SYSROOT_DIR}/usr/lib/x86_64-linux-gnu/fakechroot/libfakechroot.so" ]; then
  FAKECHROOT_LIB="${SYSROOT_DIR}/usr/lib/x86_64-linux-gnu/fakechroot/libfakechroot.so"
elif [ -f "${SYSROOT_DIR}/usr/lib/aarch64-linux-gnu/fakechroot/libfakechroot.so" ]; then
  FAKECHROOT_LIB="${SYSROOT_DIR}/usr/lib/aarch64-linux-gnu/fakechroot/libfakechroot.so"
fi
if [ -z "${FAKECHROOT_LIB}" ]; then
  echo "ERROR: libfakechroot.so not found in sysroot (tried x86_64 and aarch64 multiarch paths)" >&2
  exit 1
fi

FAKECHROOT_LIB_DIR="$(dirname "${FAKECHROOT_LIB}")"

# Determine the sysroot's ELF interpreter (ld-linux.so) and the arch-specific
# library search path using well-known Debian multiarch paths rather than a
# fragile glob search.  The sysroot binary is always executed via SYSROOT_INTERP
# so all of its dependencies (including libc.so.6) come from the sysroot —
# avoiding loading the sysroot's libc alongside the host's (two libc instances
# → segfault).
if [ -f "${SYSROOT_DIR}/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2" ]; then
  SYSROOT_INTERP="${SYSROOT_DIR}/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2"
  SYSROOT_LIBPATH="${SYSROOT_DIR}/usr/lib/x86_64-linux-gnu:${SYSROOT_DIR}/usr/lib"
elif [ -f "${SYSROOT_DIR}/usr/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1" ]; then
  SYSROOT_INTERP="${SYSROOT_DIR}/usr/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1"
  SYSROOT_LIBPATH="${SYSROOT_DIR}/usr/lib/aarch64-linux-gnu:${SYSROOT_DIR}/usr/lib"
else
  echo "ERROR: sysroot ELF interpreter not found (tried x86_64 and aarch64 paths)" >&2
  exit 1
fi

# Build the exclude paths list.
# Always exclude BUILD_WORKSPACE_DIRECTORY so workspace operations stay on host.
EXCLUDE_PATHS=""
if [ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]; then
  EXCLUDE_PATHS="${BUILD_WORKSPACE_DIRECTORY}"
fi

# Append any user-specified exclude paths.
if [ -n "${FAKECHROOT_EXCLUDE_PATH:-}" ]; then
  if [ -n "${EXCLUDE_PATHS}" ]; then
    EXCLUDE_PATHS="${EXCLUDE_PATHS}:${FAKECHROOT_EXCLUDE_PATH}"
  else
    EXCLUDE_PATHS="${FAKECHROOT_EXCLUDE_PATH}"
  fi
fi

# Exclude SYSROOT_INTERP (the sysroot's ld-linux.so) so fakechroot does not
# intercept its execution.  When a sysroot binary is launched via
#   exec "$SYSROOT_INTERP" --library-path "$SYSROOT_LIBPATH" "$SYSROOT_DIR/bin"
# the path "$SYSROOT_DIR/…/ld-linux.so.2" starts with FAKECHROOT_BASE.
# Fakechroot would translate it to a sysroot-relative path and fail to exec it
# (the kernel reports ENOENT).  Listing the full path here tells fakechroot to
# pass the execve through unchanged.
if [ -n "${SYSROOT_INTERP}" ]; then
  if [ -n "${EXCLUDE_PATHS}" ]; then
    EXCLUDE_PATHS="${EXCLUDE_PATHS}:${SYSROOT_INTERP}"
  else
    EXCLUDE_PATHS="${SYSROOT_INTERP}"
  fi
fi

if [ -n "${EXCLUDE_PATHS}" ]; then
  export FAKECHROOT_EXCLUDE_PATH="${EXCLUDE_PATHS}"
fi

# Wire libfakechroot.so as LD_PRELOAD and point FAKECHROOT_BASE at the sysroot
# directory.  All absolute file-system calls (open, stat, execve, …) inside the
# launched process are transparently redirected to SYSROOT_DIR/<path> unless the
# path is listed in FAKECHROOT_EXCLUDE_PATH.
export FAKECHROOT_BASE="${SYSROOT_DIR}"
export LD_PRELOAD="${FAKECHROOT_LIB}${LD_PRELOAD:+:${LD_PRELOAD}}"
# Add the fakechroot library directory to LD_LIBRARY_PATH so fakechroot's own
# shared dependencies are resolved without falling back to host libraries.
export LD_LIBRARY_PATH="${FAKECHROOT_LIB_DIR}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"

exec "${SYSROOT_INTERP}" --library-path "${SYSROOT_LIBPATH}" "${SYSROOT_DIR}${COMMAND_IN_SYSROOT}" "$@"
