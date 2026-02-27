# SBOM Tests Package
#
# This package contains tests for the SBOM generation system.

load("@rules_python//python:defs.bzl", "py_test")

package(default_visibility = ["//visibility:private"])

py_test(
    name = "test_spdx_formatter",
    srcs = ["test_spdx_formatter.py"],
    deps = ["//sbom/internal/generator:spdx_formatter"],
)

py_test(
    name = "test_cyclonedx_formatter",
    srcs = ["test_cyclonedx_formatter.py"],
    deps = ["//sbom/internal/generator:cyclonedx_formatter"],
)

py_test(
    name = "test_bcr_known_licenses",
    srcs = ["test_bcr_known_licenses.py"],
    deps = ["//sbom/internal/generator:sbom_generator_lib"],
)

py_test(
    name = "test_cpp_enrich_checksum",
    srcs = ["test_cpp_enrich_checksum.py"],
    data = ["//sbom:cpp_metadata.json"],
    deps = ["//sbom/internal/generator:sbom_generator_lib"],
)
