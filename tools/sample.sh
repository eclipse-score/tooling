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

# This script is a sample for consuming repos: paths below (e.g. //docs:ide_support,
# .venv_docs) are specific to this repo's layout and must be adapted to your own.
#
# Ruff and Yamlfmt are consumed directly from `@aspect_rules_lint`'s bundled
# multitool hub -- add `bazel_dep(name = "aspect_rules_lint", ...)` to your own
# MODULE.bazel to get `@aspect_rules_lint//lint:ruff_bin` and
# `@aspect_rules_lint//format:yamlfmt`.
# Actionlint is not bundled by aspect_rules_lint, so it's consumed from
# score_tooling's own dedicated hub instead.
bazel run //docs:ide_support

echo "Running Ruff linter..."
bazel run @aspect_rules_lint//lint:ruff_bin -- check

echo "Running basedpyright..."
.venv_docs/bin/python3 -m basedpyright

echo "Running Actionlint..."
bazel run @score_tooling//tools:actionlint

echo "Running Yamlfmt..."
bazel run @aspect_rules_lint//format:yamlfmt -- $(find . \
  -type d \( -name .git -o -name .venv -o -name bazel-out -o -name node_modules \) -prune -false \
  -o -type f \( -name "*.yaml" -o -name "*.yml" \) | tr '\n' '\0' | xargs -0)
