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
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "puml_parser",
    srcs = ["src/lib.rs"],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        ":class_diagram",
        ":component_diagram",
        ":parser_core",
        ":preprocessor",
        ":sequence_diagram",
    ],
)

alias(
    name = "class_diagram",
    actual = "//plantuml/parser/puml_parser/src/class_diagram:puml_parser_class",
    visibility = ["//plantuml/parser:__subpackages__"],
)

alias(
    name = "component_diagram",
    actual = "//plantuml/parser/puml_parser/src/component_diagram:puml_parser_component",
    visibility = ["//plantuml/parser:__subpackages__"],
)

alias(
    name = "sequence_diagram",
    actual = "//plantuml/parser/puml_parser/src/sequence_diagram:puml_parser_sequence",
    visibility = ["//plantuml/parser:__subpackages__"],
)

alias(
    name = "preprocessor",
    actual = "//plantuml/parser/puml_parser/src/preprocessor:puml_parser_preprocessor",
    visibility = ["//plantuml/parser:__subpackages__"],
)

alias(
    name = "parser_core",
    actual = "//plantuml/parser/puml_parser/src/parser_core:puml_parser_core",
    visibility = ["//plantuml/parser:__subpackages__"],
)
