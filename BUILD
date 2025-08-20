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
load("//cr_checker:cr_checker.bzl", "copyright_checker")

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
package(default_visibility = ["//visibility:public"])

copyright_checker(
    name = "copyright",
    srcs = [
        #"//tools",  # Use full label if src is a package
        "//:BUILD",
        "//:MODULE.bazel",
        "cli_helper",
        "cr_checker",
        "dash",
        "format_checker",
        "python_basics",
        "starpls",
        "tools"

        # Add other directories/files you want to check
    ],
    config = "//cr_checker/resources:config",
    template = "//cr_checker/resources:templates",
    visibility = ["//visibility:public"],
)
