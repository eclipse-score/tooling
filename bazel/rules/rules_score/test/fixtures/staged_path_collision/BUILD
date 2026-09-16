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

load(":dup_srcs_aux_fixture.bzl", "dup_srcs_aux_fixture")

package(default_visibility = ["//:__pkg__"])

# See dup_srcs_aux_fixture.bzl and :staged_path_same_label_collision_repro in
# test/BUILD.
dup_srcs_aux_fixture(
    name = "dup_srcs_aux",
    src = "content.rst",
)
