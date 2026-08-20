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
# End-to-end test of the score_tooling LLVM coverage pipeline, run against
# this consumer-style workspace. Asserts the properties the pipeline
# guarantees:
#   1. Untested in-scope files (C++ AND Rust) appear at exact 0% in the LCOV.
#   2. The effective-coverage gate fails at threshold 100 and passes at a low
#      threshold.
#   3. The justified line raises effective coverage above raw coverage.

set -euo pipefail
cd "$(dirname "$0")"

echo "=== Running coverage build ==="
bazel coverage --config=llvm_cov //... --build_tests_only

YAML="tools/coverage/coverage_justifications.yaml"

echo "=== Gate must FAIL at threshold 100 (uncovered fixtures exist) ==="
if COVERAGE_THRESHOLD=100 bazel run @score_tooling//coverage:generate_coverage_html -- \
    --yaml "${YAML}" --archive coverage_artifacts; then
  echo "ERROR: coverage gate passed at threshold 100 despite uncovered files" >&2
  exit 1
fi
echo "OK: gate failed as expected"

echo "=== Gate must PASS at a low threshold ==="
COVERAGE_THRESHOLD=10 bazel run @score_tooling//coverage:generate_coverage_html -- \
    --yaml "${YAML}"
echo "OK: gate passed as expected"

echo "=== Without --yaml: HTML still produced, gate applies to RAW coverage ==="
if COVERAGE_THRESHOLD=100 bazel run @score_tooling//coverage:generate_coverage_html; then
  echo "ERROR: raw-coverage gate passed at threshold 100" >&2
  exit 1
fi
COVERAGE_THRESHOLD=10 bazel run @score_tooling//coverage:generate_coverage_html
if [[ ! -f coverage_linux/index.html ]]; then
  echo "ERROR: HTML report missing after no-yaml run" >&2
  exit 1
fi
echo "OK: no-yaml mode works (HTML produced, raw gate enforced)"

echo "=== --archive-dir must produce an unzipped artifacts tree ==="
COVERAGE_THRESHOLD=10 bazel run @score_tooling//coverage:generate_coverage_html -- \
    --yaml "${YAML}" --archive-dir artifacts_dir
for f in artifacts_dir/coverage_linux/index.html artifacts_dir/coverage_report.dat \
         artifacts_dir/justification_report/summary.txt; do
  if [[ ! -f "$f" ]]; then
    echo "ERROR: ${f} missing from --archive-dir output" >&2
    exit 1
  fi
done
rm -rf artifacts_dir
echo "OK: --archive-dir works"

echo "=== Untested files must appear at exact 0% in the LCOV ==="
unzip -p coverage_artifacts.zip artifacts/coverage_report.dat > lcov.dat

check_zero_coverage() {
  local file="$1"
  if ! grep -q "SF:.*${file}" lcov.dat; then
    echo "ERROR: ${file} missing from LCOV (baseline mechanism broken)" >&2
    exit 1
  fi
  # The record for the file must report zero lines hit.
  if ! awk -v f="${file}" '
      $0 ~ "^SF:" && $0 ~ f {rec=1}
      rec && /^LH:/ {print $0; exit ($0 == "LH:0") ? 0 : 1}
      rec && /^end_of_record/ {exit 1}' lcov.dat; then
    echo "ERROR: ${file} is present but not at 0% coverage" >&2
    exit 1
  fi
  echo "OK: ${file} present at 0%"
}

check_zero_coverage "src/uncovered.cpp"
check_zero_coverage "rust/main.rs"

echo "=== Covered files must be present with hits ==="
grep -q "SF:.*src/coverable.cpp" lcov.dat || { echo "ERROR: coverable.cpp missing" >&2; exit 1; }
grep -q "SF:.*rust/lib.rs" lcov.dat || { echo "ERROR: lib.rs missing" >&2; exit 1; }
echo "OK"

echo "=== Justified line must raise effective coverage above raw ==="
SUMMARY="$(unzip -p coverage_artifacts.zip artifacts/justification_report/summary.txt)"
echo "${SUMMARY}"
JUSTIFIED="$(echo "${SUMMARY}" | grep -oP 'Justified lines:\s+\K[0-9]+')"
if [[ "${JUSTIFIED}" -lt 1 ]]; then
  echo "ERROR: expected at least one justified line, got ${JUSTIFIED}" >&2
  exit 1
fi
RAW="$(echo "${SUMMARY}" | grep -oP 'Raw line coverage:\s+\K[0-9.]+')"
EFFECTIVE="$(echo "${SUMMARY}" | grep -oP 'Effective line coverage:\s+\K[0-9.]+')"
if ! awk "BEGIN {exit (${EFFECTIVE} > ${RAW}) ? 0 : 1}"; then
  echo "ERROR: effective coverage ${EFFECTIVE}% not above raw ${RAW}%" >&2
  exit 1
fi
echo "OK: effective ${EFFECTIVE}% > raw ${RAW}%"

echo ""
echo "=== All integration checks passed ==="
