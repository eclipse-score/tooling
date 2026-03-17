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
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "puml_parser_preprocessor",
    srcs = [
        "src/lib.rs",
        "src/preprocessor.rs",
    ],
    crate_name = "preprocessor",
    crate_root = "src/lib.rs",
    proc_macro_deps = [
        "@crates//:pest_derive",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_parser/src/preprocessor/src/include:include_preprocessor",
        "//plantuml/parser/puml_parser/src/preprocessor/src/procedure:procedure_preprocessor",
        "//plantuml/parser/puml_utils",
        "@crates//:log",
        "@crates//:thiserror",
    ],
)

rust_test(
    name = "puml_parser_include_preprocessor_it",
    srcs = [
        "tests/include_tests.rs",
        "tests/preprocess_runner.rs",
    ],
    crate_root = "tests/include_tests.rs",
    data = [
        "//plantuml/parser/puml_parser/tests/preprocessor/include:include_preprocessor_files",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        ":puml_parser_preprocessor",
        "//plantuml/parser/integration_test:test_framework",
        "//plantuml/parser/puml_utils",
        "@crates//:serde_yaml",
    ],
)

rust_test(
    name = "puml_parser_procedure_preprocessor_it",
    srcs = [
        "tests/preprocess_runner.rs",
        "tests/procedure_tests.rs",
    ],
    crate_root = "tests/procedure_tests.rs",
    data = [
        "//plantuml/parser/puml_parser/tests/preprocessor/procedure:procedure_files",
    ],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        ":puml_parser_preprocessor",
        "//plantuml/parser/integration_test:test_framework",
        "//plantuml/parser/puml_utils",
        "@crates//:serde_yaml",
    ],
)
