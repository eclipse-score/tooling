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

load(":architecture_labels_test.bzl", "architecture_labels_test_suite")
load(":certified_scope_test.bzl", "certified_scope_index_test_suite", "certified_scope_matching_test_suite")
load(":label_path_test.bzl", "label_path_test_suite")
load(":lobster_config_test.bzl", "lobster_config_test_suite")
load(":puml_parser_test.bzl", "puml_parser_test_suite")
load(":puml_utils_test.bzl", "puml_utils_test_suite")
load(":tooling_provenance_test.bzl", "tooling_provenance_test_suite")

# L1: tests of pure Starlark logic in private/*.bzl (skylib loadingtest).
architecture_labels_test_suite(name = "architecture_labels_test")

certified_scope_matching_test_suite(name = "certified_scope_matching_test")

certified_scope_index_test_suite(name = "certified_scope_index_test")

label_path_test_suite(name = "label_path_test")

lobster_config_test_suite(name = "lobster_config_test")

puml_parser_test_suite(name = "puml_parser_test")

puml_utils_test_suite(name = "puml_utils_test")

tooling_provenance_test_suite(name = "tooling_provenance_test")

test_suite(name = "l1")
