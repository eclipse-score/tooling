#!/usr/bin/env bash
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

# Shared sh_test helper: locate a runfile by a suffix match against its full
# path. dependable_element generates many same-named files (one index.rst per
# architectural_design view/directory plus its own top-level index), so a bare
# basename match is ambiguous.
#
# Usage: find_runfile SUFFIX PATH...
#   SUFFIX: the trailing portion of the desired file's path to match against.
#   PATH...: candidate `$(rootpaths ...)`-style runfiles-relative paths.
#
# Echoes the first matching candidate's absolute path (resolved via
# TEST_SRCDIR/TEST_WORKSPACE) and returns 0, or prints an error to stderr and
# returns 1 if none of the candidates match.
find_runfile() {
    local suffix="$1"
    shift
    local rel_path candidate
    for rel_path in "$@"; do
        candidate="${TEST_SRCDIR}/${TEST_WORKSPACE}/${rel_path}"
        if [[ -f "${candidate}" && "${candidate}" == *"${suffix}" ]]; then
            echo "${candidate}"
            return 0
        fi
    done
    echo "Error: could not locate '*${suffix}' among: $*" >&2
    return 1
}
