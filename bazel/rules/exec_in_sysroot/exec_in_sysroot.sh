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
#   exec_in_sysroot.sh <host-executable> [arguments...]
#
# Environment variables (set by the generated exec_in_sysroot wrapper):
#   SYSROOT_DIR             - Path to the extracted sysroot directory (required)
#   FAKECHROOT_EXCLUDE_PATH - Colon-separated paths NOT to redirect (the host
#                             executable and its runfiles dir are pre-excluded)
#   BUILD_WORKSPACE_DIRECTORY - Workspace root, automatically excluded (optional)

set -eu

COMMAND_IN_SYSROOT="${1:?Command must be provided as first argument}"
shift || true

SYSROOT_DIR="${SYSROOT_DIR:?SYSROOT_DIR must be set}"

FAKECHROOT_LIB=""
if [ -f "${SYSROOT_DIR}/usr/lib/x86_64-linux-gnu/fakechroot/libfakechroot.so" ]; then
  FAKECHROOT_LIB="${SYSROOT_DIR}/usr/lib/x86_64-linux-gnu/fakechroot/libfakechroot.so"
elif [ -f "${SYSROOT_DIR}/usr/lib/aarch64-linux-gnu/fakechroot/libfakechroot.so" ]; then
  FAKECHROOT_LIB="${SYSROOT_DIR}/usr/lib/aarch64-linux-gnu/fakechroot/libfakechroot.so"
fi
if [ -z "${FAKECHROOT_LIB}" ]; then
  FAKECHROOT_LIB="$(find "${SYSROOT_DIR}/usr/lib" -path '*/fakechroot/libfakechroot.so' -type f | head -1 || true)"
fi
if [ -z "${FAKECHROOT_LIB}" ]; then
  echo "ERROR: libfakechroot.so not found in sysroot" >&2
  exit 1
fi

FAKECHROOT_LIB_DIR="$(dirname "${FAKECHROOT_LIB}")"

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

if [ -n "${EXCLUDE_PATHS}" ]; then
  export FAKECHROOT_EXCLUDE_PATH="${EXCLUDE_PATHS}"
fi

# Wire libfakechroot.so as LD_PRELOAD and point FAKECHROOT_BASE at the sysroot
# directory.  All absolute file-system calls (open, stat, execve, …) inside the
# launched process are transparently redirected to SYSROOT_DIR/<path> unless the
# path is listed in FAKECHROOT_EXCLUDE_PATH.
#
# Only the fakechroot-lib directory is added to LD_LIBRARY_PATH; adding the full
# sysroot lib tree would make the host linker pick up sysroot-snapshot versions of
# libgvc/libc/… which may differ from the build the sysroot binary was built with
# and cause crashes.
export FAKECHROOT_BASE="${SYSROOT_DIR}"
export LD_PRELOAD="${FAKECHROOT_LIB}${LD_PRELOAD:+:${LD_PRELOAD}}"
export LD_LIBRARY_PATH="${FAKECHROOT_LIB_DIR}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"

exec "${COMMAND_IN_SYSROOT}" "$@"
