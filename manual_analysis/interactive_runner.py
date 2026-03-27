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
"""Interactive manual-analysis runner.

This module is intentionally a compatibility facade. The implementation is
split across focused modules to reduce coupling and improve maintainability.
"""

from __future__ import annotations

try:
    from manual_analysis.interactive_runner_cli import _create_parser, main
    from manual_analysis.interactive_runner_flow import run_analysis
    from manual_analysis.interactive_runner_prefill import _PrefillState
    from manual_analysis.interactive_runner_runtime import (
        _install_signal_handlers,
        _interrupt_exit_code,
        _signal_handler,
        _workspace_root,
    )
    from manual_analysis.interactive_runner_steps import (
        AnalysisFailedError,
        _execute_step,
    )
    from manual_analysis.interactive_runner_ui_split import _SplitPaneUI
    from manual_analysis.yaml_schema import (
        ActionStep,
        AssertionStep,
        AutomatedActionArg,
        AutomatedActionStep,
        DecisionStep,
        RepeatStep,
        Step,
        load_analysis,
    )
except ModuleNotFoundError:
    from interactive_runner_cli import _create_parser, main
    from interactive_runner_flow import run_analysis
    from interactive_runner_prefill import _PrefillState
    from interactive_runner_runtime import (
        _install_signal_handlers,
        _interrupt_exit_code,
        _signal_handler,
        _workspace_root,
    )
    from interactive_runner_steps import AnalysisFailedError, _execute_step
    from interactive_runner_ui_split import _SplitPaneUI
    from yaml_schema import (
        ActionStep,
        AssertionStep,
        AutomatedActionArg,
        AutomatedActionStep,
        DecisionStep,
        RepeatStep,
        Step,
        load_analysis,
    )


if __name__ == "__main__":
    main()
