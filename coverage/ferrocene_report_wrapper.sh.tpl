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

set -euo pipefail

report_runfile=@@RUNFILE@@
if [[ -n "${RUNFILES_DIR:-}" ]]; then
  ferrocene_report="${RUNFILES_DIR}/${report_runfile}"
elif [[ -n "${RUNFILES_MANIFEST_FILE:-}" ]]; then
  ferrocene_report="$(grep -m1 "^${report_runfile} " "${RUNFILES_MANIFEST_FILE}" | cut -d' ' -f2-)"
else
  ferrocene_report="${report_runfile}"
fi

if [[ ! -x "${ferrocene_report}" ]]; then
  echo "ferrocene_report not found at ${ferrocene_report}" >&2
  exit 1
fi

@@EXEC_LINE@@
