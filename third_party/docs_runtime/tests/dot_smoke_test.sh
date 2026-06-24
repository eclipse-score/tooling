#!/usr/bin/env bash
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
#
# Smoke test for the hermetic graphviz setup:
#   //third_party/docs_runtime:dot — exec_in_sysroot wrapper executing /usr/bin/dot
#                                    inside the docs_runtime sysroot.
# If this wrapper/sysroot pairing breaks, doc builds lose graphviz rendering.

set -euo pipefail

# --- begin runfiles.bash initialization ---
if [[ ! -d "${RUNFILES_DIR:-/dev/null}" && ! -f "${RUNFILES_MANIFEST_FILE:-/dev/null}" ]]; then
  if [[ -f "$0.runfiles_manifest" ]]; then
    export RUNFILES_MANIFEST_FILE="$0.runfiles_manifest"
  elif [[ -f "$0.runfiles/MANIFEST" ]]; then
    export RUNFILES_MANIFEST_FILE="$0.runfiles/MANIFEST"
  elif [[ -f "$0.runfiles/bazel_tools/tools/bash/runfiles/runfiles.bash" ]]; then
    export RUNFILES_DIR="$0.runfiles"
  fi
fi
if [[ -f "${RUNFILES_DIR:-/dev/null}/bazel_tools/tools/bash/runfiles/runfiles.bash" ]]; then
  source "${RUNFILES_DIR}/bazel_tools/tools/bash/runfiles/runfiles.bash"
elif [[ -f "${RUNFILES_MANIFEST_FILE:-/dev/null}" ]]; then
  source "$(grep -m1 '^bazel_tools/tools/bash/runfiles/runfiles.bash ' "$RUNFILES_MANIFEST_FILE" | cut -d ' ' -f 2-)"
else
  echo >&2 "ERROR: cannot find @bazel_tools//tools/bash/runfiles:runfiles.bash"
  exit 1
fi
# --- end runfiles.bash initialization ---

dot_wrapper="$(rlocation "${TEST_WORKSPACE}/third_party/docs_runtime/dot")"
if [[ -z "${dot_wrapper}" || ! -x "${dot_wrapper}" ]]; then
  echo "ERROR: could not resolve //third_party/docs_runtime:dot from runfiles" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# 1. Sanity: version string.
# ---------------------------------------------------------------------------
echo "=== dot -V ==="
"${dot_wrapper}" -V 2>&1

# ---------------------------------------------------------------------------
# 2. Render a minimal digraph to SVG and verify the output contains valid SVG.
# ---------------------------------------------------------------------------
echo "=== rendering digraph to SVG ==="
svg="$(printf 'digraph G { A -> B }' | "${dot_wrapper}" -Tsvg 2>&1)"

if [[ "${svg}" != *"<svg"* ]]; then
    echo "ERROR: dot produced no SVG output" >&2
    echo "--- output ---" >&2
    echo "${svg}" >&2
    exit 1
fi

echo "ok: hermetic dot works"
