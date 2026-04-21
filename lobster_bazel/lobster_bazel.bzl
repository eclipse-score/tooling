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
load("//lobster_bazel/private:lobster_linker.bzl", _lobster_linker = "lobster_linker", _subrule_lobster_linker = "subrule_lobster_linker")

# Re-export LobsterProvider so it can be loaded from this file
def lobster_linker(**kwargs):
    _lobster_linker(**kwargs)

subrule_lobster_linker = _subrule_lobster_linker
