# SBOM Generator Python package
#
# This package contains the Python tools for generating SBOM files
# in SPDX 2.3 and CycloneDX 1.6 formats.

load("@rules_python//python:defs.bzl", "py_binary", "py_library")

package(default_visibility = ["//sbom:__subpackages__"])

py_binary(
    name = "sbom_generator",
    srcs = ["sbom_generator.py"],
    main = "sbom_generator.py",
    deps = [
        ":cyclonedx_formatter",
        ":spdx_formatter",
    ],
)

py_library(
    name = "sbom_generator_lib",
    srcs = ["sbom_generator.py"],
    deps = [
        ":cyclonedx_formatter",
        ":spdx_formatter",
    ],
)

py_library(
    name = "spdx_formatter",
    srcs = ["spdx_formatter.py"],
)

py_library(
    name = "cyclonedx_formatter",
    srcs = ["cyclonedx_formatter.py"],
)
