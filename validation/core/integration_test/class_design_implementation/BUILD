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

load("@rules_rust//rust:defs.bzl", "rust_test")

filegroup(
    name = "class_design_implementation_test_data",
    srcs = [
        "//validation/core/integration_test/class_design_implementation/negative_class_member_missing:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_class_missing:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_entity_type_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_enum_literal_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_enum_literal_missing:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_method_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_method_modifier_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_method_parameter_count_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_method_parameter_name_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_method_parameter_type_mismatch:case_data",
        # "//validation/core/integration_test/class_design_implementation/negative_method_parameter_variadic_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_method_visibility_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_method_missing:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_relationship_missing:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_relationship_type_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_template_parameter_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_template_parameter_pack_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_type_alias_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_type_alias_missing:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_variable_static_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_variable_visibility_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/negative_variable_mismatch:case_data",
        "//validation/core/integration_test/class_design_implementation/positive_class_features:case_data",
        "//validation/core/integration_test/class_design_implementation/positive_method_features:case_data",
        "//validation/core/integration_test/class_design_implementation/positive_method_template_pack_features:case_data",
        "//validation/core/integration_test/class_design_implementation/positive_relationship_features:case_data",
        "//validation/core/integration_test/class_design_implementation/positive_variable_features:case_data",
    ],
)

rust_test(
    name = "class_design_implementation_integration_test",
    srcs = ["class_design_implementation_suite.rs"],
    crate_root = "class_design_implementation_suite.rs",
    data = [
        ":class_design_implementation_test_data",
        "//validation/core:validation_cli",
    ],
    deps = [
        "//validation/core:validation",
        "//validation/core/integration_test:test_framework",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
