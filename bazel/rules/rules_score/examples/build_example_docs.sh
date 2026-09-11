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
# Builds the dependable_element documentation of every example and stages the
# resulting HTML under <out-dir>/<example>/.
#
# Every example is a standalone Bazel module that reaches back into this repo
# via local_path_override. The root module therefore cannot depend on them --
# that would close a bazel_dep cycle -- so their docs are built out-of-band
# here and copied into the published docs tree next to the Sphinx output of
# //bazel/rules/rules_score:rules_score_doc.
#
# Usage:
#   bazel/rules/rules_score/examples/build_example_docs.sh <out-dir>
set -euo pipefail

# "<example dir>:<dependable_element doc target>"
EXAMPLES=(
  "minimal:my_element_doc"
  "some_other_library:other_seooc_doc"
  "seooc:safety_software_seooc_example_doc"
  "integrator:integrator_seooc_doc"
)

if [[ $# -ne 1 ]]; then
  echo "usage: $(basename "$0") <out-dir>" >&2
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out_dir="$(mkdir -p "$1" && cd "$1" && pwd)"

for entry in "${EXAMPLES[@]}"; do
  example="${entry%%:*}"
  target="${entry#*:}"

  echo "==> Building documentation for example '${example}'"
  (
    cd "${script_dir}/${example}"
    bazel build "//:${target}"

    html_dir="$(bazel info bazel-bin)/${target}/html"
    if [[ ! -d "${html_dir}" ]]; then
      echo "error: expected HTML output not found at ${html_dir}" >&2
      exit 1
    fi

    rm -rf "${out_dir:?}/${example}"
    mkdir -p "${out_dir}/${example}"
    cp -r "${html_dir}/." "${out_dir}/${example}/"
    chmod -R u+w "${out_dir}/${example}"

    # Each example runs its own Bazel server; keeping five alive at once
    # exhausts memory on standard CI runners.
    bazel shutdown
  )
done

echo "==> Example documentation staged in ${out_dir}"
