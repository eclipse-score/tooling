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
    name = "puml_resolver",
    srcs = ["src/lib.rs"],
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        ":class_resolver",
        ":component_resolver",
        ":resolver_traits",
    ],
)

alias(
    name = "component_resolver",
    actual = "//plantuml/parser/puml_resolver/src/component_diagram:puml_resolver_component",
    visibility = ["//plantuml/parser:__subpackages__"],
)

alias(
    name = "class_resolver",
    actual = "//plantuml/parser/puml_resolver/src/class_diagram:puml_resolver_class",
    visibility = ["//plantuml/parser:__subpackages__"],
)

rust_library(
    name = "resolver_traits",
    srcs = [
        "src/resolver_traits.rs",
    ],
    crate_name = "resolver_traits",
    crate_root = "src/resolver_traits.rs",
    visibility = ["//plantuml/parser:__subpackages__"],
    deps = [
        "@crates//:log",
    ],
)
