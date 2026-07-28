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
# Generic wrapper script that executes a command inside a sysroot using fakechroot.
#
# Rather than using `fakechroot -- chroot SYSROOT COMMAND` (which relies on the
# host's dynamic linker and would cause the host libc and the sysroot libc to be
# loaded simultaneously, resulting in a segfault), this script directly invokes the
# sysroot's own ELF interpreter (ld-linux.so) with --library-path pointing at the
# sysroot's library tree.  This guarantees a single-libc environment: all shared
# libraries, including libc, come exclusively from the sysroot.  fakechroot is still
# loaded via LD_PRELOAD so that absolute file-system calls are transparently
# redirected into the sysroot, but the chroot simulation itself is bypassed entirely.
#
# WHY NOT `fakechroot -- chroot SYSROOT $SYSROOT_INTERP --library-path ...`?
#
# One might try to preserve the fakechroot binary invocation (in case it does more
# than just set LD_PRELOAD) while still using the sysroot's own ld-linux.so.  This
# does not work cleanly for the following reason:
#
#   ld-linux.so.2 is the dynamic linker itself.  When invoked as a standalone
#   executable it bootstraps before glibc exists and performs its own file I/O
#   via raw kernel syscalls, NOT via glibc wrappers.  Because fakechroot works
#   exclusively through LD_PRELOAD (hooking glibc wrappers), it cannot intercept
#   ld-linux.so.2's internal library lookups.  The --library-path argument passed
#   to ld-linux.so.2 is therefore resolved against the real host filesystem.  If
#   relative (chroot-relative) paths are used, ld-linux.so.2 loads host libraries
#   → dual libc → segfault.  If absolute sysroot paths are used instead, fakechroot
#   would double-translate them (prepend SYSROOT_DIR a second time) unless every
#   such path is individually listed in FAKECHROOT_EXCLUDE_PATH.
#
#   The fakechroot binary itself adds little beyond setting LD_PRELOAD and
#   FAKECHROOT_BASE plus substituting package-management commands (ldconfig, chown,
#   etc.) that are irrelevant when running a plain ELF binary.  Setting those two
#   environment variables directly (as this script does) is fully equivalent for
#   this use case.
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
# _append_exclude appends its argument to EXCLUDE_PATHS (colon-separated),
# skipping empty values, so callers below don't each need their own
# if-non-empty-then-append-else-init boilerplate.
EXCLUDE_PATHS=""
_append_exclude() {
  [ -n "$1" ] || return 0
  if [ -n "${EXCLUDE_PATHS}" ]; then
    EXCLUDE_PATHS="${EXCLUDE_PATHS}:$1"
  else
    EXCLUDE_PATHS="$1"
  fi
}

# Always exclude BUILD_WORKSPACE_DIRECTORY so workspace operations stay on host.
_append_exclude "${BUILD_WORKSPACE_DIRECTORY:-}"

# Always exclude the sandbox execroot too, derived from RUNFILES_DIR /
# RUNFILES_MANIFEST_FILE.  BUILD_WORKSPACE_DIRECTORY is only set for
# `bazel run`/`bazel test`; plain `bazel build` actions leave it unset, and
# tools invoked deep inside the action (e.g. Sphinx's sphinx.ext.graphviz,
# which calls the hermetic `dot` with `-o<abs-path-under-bazel-out>` while
# running it with cwd set to the *source* doc directory, not the execroot)
# still need their absolute, execroot-relative writes to land on the real
# filesystem instead of being redirected into SYSROOT_DIR.
# RUNFILES_DIR (or the runfiles manifest's
# directory) is set once per action and always sits at
# "<execroot>/bazel-out/<config>/.../<target>.runfiles" — a stable Bazel
# layout invariant — so stripping everything from "/bazel-out/" onward
# yields the execroot root regardless of the caller's cwd. This is the
# launcher's own RUNFILES_DIR (exec_in_sysroot_launcher.sh.tpl rebinds it to
# "$0.runfiles" before exec'ing this script) or, failing that, whatever
# RUNFILES_DIR/RUNFILES_MANIFEST_FILE this process inherited -- both live
# under the same sandbox execroot as the tool's real inputs/outputs.
RUNFILES_HINT="${RUNFILES_DIR:-}"
if [ -z "${RUNFILES_HINT}" ] && [ -n "${RUNFILES_MANIFEST_FILE:-}" ]; then
  RUNFILES_HINT="${RUNFILES_MANIFEST_FILE}"
fi
case "${RUNFILES_HINT}" in
  */bazel-out/*)
    _append_exclude "${RUNFILES_HINT%%/bazel-out/*}"
    ;;
esac

# Append any user-specified exclude paths.
_append_exclude "${FAKECHROOT_EXCLUDE_PATH:-}"

# Exclude SYSROOT_INTERP (the sysroot's ld-linux.so) so fakechroot does not
# intercept its execution.  When a sysroot binary is launched via
#   exec "$SYSROOT_INTERP" --library-path "$SYSROOT_LIBPATH" "$SYSROOT_DIR/bin"
# the path "$SYSROOT_DIR/…/ld-linux.so.2" starts with FAKECHROOT_BASE.
# Fakechroot would translate it to a sysroot-relative path and fail to exec it
# (the kernel reports ENOENT).  Listing the full path here tells fakechroot to
# pass the execve through unchanged.
_append_exclude "${SYSROOT_INTERP}"

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
