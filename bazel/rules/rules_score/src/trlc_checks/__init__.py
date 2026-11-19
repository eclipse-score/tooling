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

"""TRLC check plugins for ``trlc_check_runner``.

A plugin module defines ``check(symbols, mh) -> bool``. It only runs when the
caller names it with ``--check``; ``dependable_element`` lists them.
"""


def model_type_names(record_type):
    """Names of the ScoreReq types the record type is or extends."""
    names = set()
    while record_type is not None:
        if record_type.n_package.name == "ScoreReq":
            names.add(record_type.name)
        record_type = record_type.parent
    return names
