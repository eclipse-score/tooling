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
filegroup(
    name = "simple_sequence",
    srcs = ["simple_sequence.puml"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "comprehensive_sequence_test",
    srcs = [
        "comprehensive_sequence_test.json",
        "comprehensive_sequence_test.puml",
    ],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "sequence_diagram_tests",
    srcs = [
        ":comprehensive_sequence_test",
        ":simple_sequence",
    ],
    visibility = ["//visibility:public"],
)
