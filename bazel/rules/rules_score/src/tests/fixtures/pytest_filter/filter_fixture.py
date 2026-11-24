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
"""Fixture of pytest_filter_test: only the test named by the filter may run."""


def test_selected() -> None:
    """Runs, because the filter selects it."""


def test_not_selected() -> None:
    """Fails, unless the filter deselects it."""
    raise AssertionError("--test_filter did not deselect this test")
