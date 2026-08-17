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
# Generates an HTML coverage report from `bazel coverage` output produced by
# the LLVM pipeline: the custom reporter produces a zip with html_report/,
# lcov_report/ and text_report/ inside.
#
# Usage (from the CONSUMER workspace):
#   bazel coverage --config=llvm_cov //... --build_tests_only
#   bazel run @score_tooling//coverage:generate_coverage_html -- \
#       --yaml <path/to/coverage_justifications.yaml> \
#       [--archive <archive-name>] [--platform <platform>] \
#       [--testlogs-subdir <subdir>] [output-dir]
#
# Arguments:
#   --yaml <path>              Justification YAML, relative to the workspace
#                              root (required).
#   --archive <archive-name>   Also create a zip archive named <archive-name>.zip
#                              containing the HTML report, raw LCOV data and JUnit XMLs.
#   --platform <platform>      Target platform for justification filtering
#                              (default: linux). Also affects the default output
#                              directory (coverage_<platform>).
#   --testlogs-subdir <subdir> Subdirectory of bazel-testlogs to collect JUnit
#                              XMLs from when archiving (default: entire
#                              bazel-testlogs tree).
#   output-dir                 Directory to write the HTML report to
#                              (default: coverage_<platform>)
#
# Environment:
#   COVERAGE_THRESHOLD        Minimum effective line coverage percentage
#                             (default: 100). The script exits non-zero when
#                             effective coverage is below this threshold.

set -euo pipefail

ARCHIVE_NAME=""
PLATFORM="linux"
OUTPUT_DIR=""
JUSTIFICATION_YAML_REL=""
TESTLOGS_SUBDIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --yaml)
      JUSTIFICATION_YAML_REL="${2:?--yaml requires a path argument}"
      shift 2
      ;;
    --archive)
      ARCHIVE_NAME="${2:?--archive requires a name argument}"
      shift 2
      ;;
    --platform)
      PLATFORM="${2:?--platform requires a platform argument (e.g. linux or qnx)}"
      shift 2
      ;;
    --testlogs-subdir)
      TESTLOGS_SUBDIR="${2:?--testlogs-subdir requires a path argument}"
      shift 2
      ;;
    *)
      OUTPUT_DIR="$1"
      shift
      ;;
  esac
done

if [[ -z "${JUSTIFICATION_YAML_REL}" ]]; then
  echo "ERROR: --yaml <path/to/coverage_justifications.yaml> is required." >&2
  exit 1
fi

# Set default output directory based on platform if not explicitly provided.
if [[ -z "${OUTPUT_DIR}" ]]; then
    OUTPUT_DIR="coverage_${PLATFORM}"
fi

# Change to the workspace root so that all subsequent bazel calls and
# relative paths work correctly.
cd "${BUILD_WORKSPACE_DIRECTORY}"

# Resolve OUTPUT_DIR to absolute path (relative to workspace root).
OUTPUT_DIR="${BUILD_WORKSPACE_DIRECTORY}/${OUTPUT_DIR}"

# The coverage report is at _coverage_report.dat: a zip containing
# html_report/, lcov_report/ and text_report/ (LLVM pipeline).
COVERAGE_REPORT="${BUILD_WORKSPACE_DIRECTORY}/bazel-out/_coverage/_coverage_report.dat"

if [[ ! -f "${COVERAGE_REPORT}" ]]; then
  echo "ERROR: Coverage report not found at ${COVERAGE_REPORT}" >&2
  echo "       Run 'bazel coverage --config=llvm_cov //... --build_tests_only' first." >&2
  exit 1
fi

TMPDIR_EXTRACT="${TMPDIR:-/tmp}/coverage_extract_$$"
mkdir -p "${TMPDIR_EXTRACT}"
trap 'rm -rf "${TMPDIR_EXTRACT}"' EXIT

rm -rf "${OUTPUT_DIR}"

if ! file -b "${COVERAGE_REPORT}" | grep -q "Zip archive"; then
  echo "ERROR: ${COVERAGE_REPORT} is not the LLVM pipeline zip report." >&2
  echo "       Run 'bazel coverage --config=llvm_cov //... --build_tests_only' first." >&2
  exit 1
fi

# Extract HTML from the zip produced by our custom reporter.
unzip -q -o "${COVERAGE_REPORT}" -d "${TMPDIR_EXTRACT}"

if [[ -d "${TMPDIR_EXTRACT}/html_report" ]]; then
  cp -r "${TMPDIR_EXTRACT}/html_report" "${OUTPUT_DIR}"
else
  echo "ERROR: html_report/ not found in ${COVERAGE_REPORT}" >&2
  exit 1
fi

echo "Coverage report written to: ${OUTPUT_DIR}"

# ---------------------------------------------------------------------------
# Run coverage justification processing.
# ---------------------------------------------------------------------------
JUSTIFICATION_YAML="${BUILD_WORKSPACE_DIRECTORY}/${JUSTIFICATION_YAML_REL}"

if [[ ! -f "${JUSTIFICATION_YAML}" ]]; then
  echo "ERROR: ${JUSTIFICATION_YAML} not found." >&2
  exit 1
fi

echo ""
echo "Running coverage justification processing..."

JUSTIFICATION_DIR="${TMPDIR_EXTRACT}/justification_report"
mkdir -p "${JUSTIFICATION_DIR}"

# Run justify.py / effective_coverage.py via nested bazel invocations from the
# consumer workspace. This deliberately avoids runfiles resolution across
# module boundaries (canonical repo names vary between Bazel versions).
bazel run @score_tooling//coverage:justify -- \
    --yaml "${JUSTIFICATION_YAML}" \
    --source-root "${BUILD_WORKSPACE_DIRECTORY}" \
    --platform "${PLATFORM}" \
    --output "${JUSTIFICATION_DIR}/manifest.json"

bazel run @score_tooling//coverage:effective_coverage -- \
    --html-dir "${OUTPUT_DIR}" \
    --manifest "${JUSTIFICATION_DIR}/manifest.json" \
    --output "${JUSTIFICATION_DIR}/report.json"

# Display effective coverage summary and enforce the threshold.
if [[ ! -f "${JUSTIFICATION_DIR}/summary.txt" ]]; then
  echo "ERROR: Effective coverage summary was not produced." >&2
  exit 1
fi

echo ""
cat "${JUSTIFICATION_DIR}/summary.txt"

# Extract effective coverage percentage for threshold check.
EFFECTIVE_PCT=$(grep -oP 'Effective line coverage:\s+\K[0-9.]+' \
  "${JUSTIFICATION_DIR}/summary.txt" 2>/dev/null || echo "0")

# Threshold check (default: 100%). Fails the run when below.
THRESHOLD="${COVERAGE_THRESHOLD:-100}"
if ! awk "BEGIN {exit (${EFFECTIVE_PCT} >= ${THRESHOLD}) ? 0 : 1}"; then
  echo "ERROR: Effective coverage ${EFFECTIVE_PCT}% is below threshold ${THRESHOLD}%" >&2
  RC=1
else
  RC=0
fi

# ---------------------------------------------------------------------------
# Optional: create a zip archive with the HTML report, raw LCOV data and
# JUnit XML test results.
# ---------------------------------------------------------------------------
if [[ -n "${ARCHIVE_NAME}" ]]; then
  mkdir -p artifacts

  # Copy JUnit XML test results preserving directory structure.
  find "bazel-testlogs/${TESTLOGS_SUBDIR}" -name 'test.xml' -exec cp --parents {} artifacts/ \;

  # Copy the HTML coverage report
  cp -r "${OUTPUT_DIR}" artifacts/

  # Include the LCOV .dat file from the reporter zip.
  if [[ -f "${TMPDIR_EXTRACT}/lcov_report/lcov.dat" ]]; then
    cp "${TMPDIR_EXTRACT}/lcov_report/lcov.dat" artifacts/coverage_report.dat
  fi

  # Include the justification report (manifest + effective coverage json).
  if [[ -d "${JUSTIFICATION_DIR}" ]]; then
    cp -r "${JUSTIFICATION_DIR}" artifacts/
  fi

  zip -r "${ARCHIVE_NAME}.zip" artifacts/
  rm -rf artifacts/
  echo "Coverage archive written to: ${ARCHIVE_NAME}.zip"
fi

exit "${RC}"
