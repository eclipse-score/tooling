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
set -euo pipefail

usage() {
  cat <<'USAGE'
Generate a combined Rust + Python coverage HTML report.

Runs 'bazel coverage' for plantuml, validation and manual_analysis, then
renders a single HTML report via genhtml.

Usage:
  bazel run //coverage:combined_report -- [options]

Options:
  --out-dir <path>    Output directory for the HTML report.
                      Default: <workspace>/coverage-html
  --targets <labels>  Space-separated list of Bazel target patterns.
                      Default: //plantuml/... //validation/... //manual_analysis/...
  --help              Show this help.

Requirements:
  genhtml and lcov must be available either via the Bazel-managed @lcov_deb
  runfiles (automatic when run via 'bazel run //coverage:combined_report') or
  installed system-wide (apt install lcov).
USAGE
}

OUT_DIR=""
TARGETS=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      OUT_DIR="$2"; shift 2 ;;
    --targets)
      TARGETS="$2"; shift 2 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1 ;;
  esac
done

# When invoked via 'bazel run', BUILD_WORKSPACE_DIRECTORY is set to the workspace
# root. We must cd into it before calling nested bazel commands, because Bazel
# refuses to be invoked from inside its own output tree.
WORKSPACE_DIR="${BUILD_WORKSPACE_DIRECTORY:-$(bazel info workspace 2>/dev/null)}"
cd "$WORKSPACE_DIR"

if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="${WORKSPACE_DIR}/coverage-html"
fi

if [[ -z "$TARGETS" ]]; then
  TARGETS="//plantuml/... //validation/... //manual_analysis/..."
fi

# ---------------------------------------------------------------------------
# Resolve lcov tools: prefer Bazel-managed binaries from @lcov_deb runfiles
# so that no system lcov installation is required.  Fall back to PATH.
# ---------------------------------------------------------------------------
_tool_path() {
  local name="$1"
  local found=""
  # Search runfiles for the Bazel-managed binary (works regardless of the
  # canonical repo name Bazel assigns under bzlmod, e.g. +_repo_rules+lcov_deb)
  if [[ -n "${RUNFILES_DIR:-}" ]]; then
    found=$(find "${RUNFILES_DIR}" -path "*/lcov_deb/usr/bin/${name}" -type f 2>/dev/null | head -1)
  fi
  # Fall back to system PATH
  if [[ -z "${found}" ]]; then
    found=$(command -v "${name}" 2>/dev/null || true)
  fi
  echo "${found}"
}

GENHTML="$(_tool_path genhtml)"
LCOV="$(_tool_path lcov)"

if [[ -z "$GENHTML" ]]; then
  echo "ERROR: 'genhtml' not found. Run via 'bazel run //coverage:combined_report' or install 'lcov'." >&2
  exit 1
fi
if [[ -z "$LCOV" ]]; then
  echo "ERROR: 'lcov' not found. Run via 'bazel run //coverage:combined_report' or install 'lcov'." >&2
  exit 1
fi

# When using the Bazel-managed tools, set PERL5LIB so Perl finds lcovutil.pm.
if [[ -n "${RUNFILES_DIR:-}" ]]; then
  lcov_lib=$(find "${RUNFILES_DIR}" -path "*/lcov_deb/usr/lib/lcov" -type d 2>/dev/null | head -1)
  if [[ -n "${lcov_lib}" ]]; then
    export PERL5LIB="${lcov_lib}${PERL5LIB:+:${PERL5LIB}}"
  fi
fi

echo "==> Running bazel coverage --config=coverage ${TARGETS}"
# shellcheck disable=SC2086  # word-splitting of TARGETS is intentional
bazel coverage --config=coverage $TARGETS

DAT_FILE="$(bazel info output_path)/_coverage/_coverage_report.dat"

if [[ ! -f "$DAT_FILE" ]]; then
  echo "ERROR: Coverage data not found at ${DAT_FILE}" >&2
  echo "       Make sure at least one test ran successfully." >&2
  exit 1
fi

echo "==> Generating HTML report in ${OUT_DIR}"
mkdir -p "$OUT_DIR"

# Remove files that should not count towards coverage:
# - external/ : Python files from rules_python internals captured by coverage.py
# lcov --ignore-errors unused prevents a failure when none of the patterns match.
FILTERED_DAT="${OUT_DIR}/filtered_coverage.dat"
"$LCOV" --remove "$DAT_FILE" \
  "external/*" \
  --output-file "$FILTERED_DAT" \
  --ignore-errors unused

"$GENHTML" "$FILTERED_DAT" \
  --output-directory "$OUT_DIR" \
  --legend \
  --title "Combined Rust + Python Coverage" \
  --rc genhtml_hi_limit=95 \
  --ignore-errors source

echo ""
echo "Coverage report: ${OUT_DIR}/index.html"
