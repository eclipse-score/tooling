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

import unittest

from manual_analysis import interactive_runner
from manual_analysis.interactive_runner_flow import (
    run_analysis as flow_run_analysis,
)
from manual_analysis.interactive_runner_prefill import (
    _PrefillState as prefill_state,
)


class InteractiveRunnerFacadeTest(unittest.TestCase):
    def test_facade_reexports_key_symbols(self) -> None:
        self.assertIs(interactive_runner.run_analysis, flow_run_analysis)
        self.assertIs(interactive_runner._PrefillState, prefill_state)
        self.assertTrue(callable(interactive_runner.main))
        self.assertTrue(callable(interactive_runner._create_parser))


if __name__ == "__main__":
    unittest.main()
