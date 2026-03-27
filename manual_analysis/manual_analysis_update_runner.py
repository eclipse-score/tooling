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
"""Run interactive manual analysis and refresh the lock file in one flow."""

from __future__ import annotations

import sys

from manual_analysis.interactive_runner_cli import main as interactive_runner_main
from manual_analysis.update_lock import main as update_lock_main


def main(argv: list[str] | None = None) -> None:
    interactive_runner_main(argv)
    update_lock_main([])


if __name__ == "__main__":
    main(sys.argv[1:])

