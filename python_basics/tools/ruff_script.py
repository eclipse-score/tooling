# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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

import subprocess
import sys
import os
from pathlib import Path


# True = 'format' else 'check'
print("sys.argv:", sys.argv)

args = []
format_flag = sys.argv[1]
action = "format" if format_flag.lower() in ("true", "1") else "check"

tool_path = ["bazel", "run", "@multitool//tools/ruff:cwd", action]

# Accept all other arguments if there are some
args.extend(sys.argv[3:])


# Is absoulte needed / right here?
# config_path = Path(os.path.realpath(__file__)).parent.parent / "pyproject.toml"

cmd = tool_path + args
# [f"--config {config_path}"]
proc = subprocess.run(cmd, capture_output=True, text=True)
