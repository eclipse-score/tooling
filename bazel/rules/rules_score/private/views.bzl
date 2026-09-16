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

"""Shared architectural_design view definitions.

Both architectural_design.bzl (the producer) and dependable_element.bzl (the
consumer, generating software_arch.rst's per-view subsections) need the same
static/dynamic/public_api/internal_api view names, in the same order, mapped
to the same display titles -- kept here once so the two can never drift.
"""

# Views recognized by architectural_design, in display order, mapped to the
# title used both as that view's top-level navigation index page heading
# (architectural_design.bzl) and its software_arch.rst subsection heading
# (dependable_element.bzl).
ARCH_VIEWS = [
    ("static", "Static Design"),
    ("dynamic", "Dynamic Design"),
    ("public_api", "Public API"),
    ("internal_api", "Internal API"),
]
