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
    name = "puml_serialize_component",
    srcs = [
        "component_serializer.rs",
    ],
    crate_name = "component_serializer",
    crate_root = "component_serializer.rs",
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_resolver:component_resolver",
        "//plantuml/parser/puml_serializer/src/fbs:component_fbs",
        "@crates//:flatbuffers",
    ],
)

rust_library(
    name = "puml_serialize_class",
    srcs = [
        "class_serializer.rs",
    ],
    crate_name = "class_serializer",
    crate_root = "class_serializer.rs",
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "//plantuml/parser/puml_serializer/src/fbs:class_fbs",
        "//tools/metamodel:class_diagram",
        "@crates//:flatbuffers",
    ],
)
