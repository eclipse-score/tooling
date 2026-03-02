# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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

load("@score_tooling//python_basics:defs.bzl", "score_py_pytest")

package(default_visibility = ["//visibility:private"])

score_py_pytest(
    name = "test_spdx_formatter",
    srcs = ["test_spdx_formatter.py"],
    deps = ["//sbom/internal/generator:spdx_formatter"],
)

score_py_pytest(
    name = "test_cyclonedx_formatter",
    srcs = ["test_cyclonedx_formatter.py"],
    deps = ["//sbom/internal/generator:cyclonedx_formatter"],
)

score_py_pytest(
    name = "test_bcr_known_licenses",
    srcs = ["test_bcr_known_licenses.py"],
    deps = ["//sbom/internal/generator:sbom_generator_lib"],
)

score_py_pytest(
    name = "test_cpp_enrich_checksum",
    srcs = ["test_cpp_enrich_checksum.py"],
    data = ["//sbom:cpp_metadata.json"],
    deps = ["//sbom/internal/generator:sbom_generator_lib"],
)
