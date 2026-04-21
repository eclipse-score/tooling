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
from pathlib import Path

from lobster.test.lobster_bazel.lobster_bazel_test_runner import LobsterBazelTestRunner
from tests_system.system_test_case_base import SystemTestCaseBase


class TestRunner:
    def __init__(self, working_dir: Path):
        self._working_dir = working_dir


class LobsterBazelSystemTestCaseBase(SystemTestCaseBase):
    def __init__(self, methodName):
        super().__init__(methodName)
        self._data_directory = Path(__file__).parents[0] / "data"

    def create_test_runner(self) -> LobsterBazelTestRunner:
        tool_name = Path(__file__).parents[0].name
        test_runner = LobsterBazelTestRunner(
            self.create_temp_dir(prefix=f"test-{tool_name}-"),
        )
        return test_runner
