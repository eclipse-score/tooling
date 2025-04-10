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

tool_path = [
    "bazel",
    "run",
    "@aspect_rules_lint//format:ruff",
    "--",
]


# Is absoulte needed / right here?
# config_path = Path(os.path.realpath(__file__)).parent.parent / "pyproject.toml"

cmd = tool_path + sys.argv[1:]
print("cmd:", " ".join(cmd))
# [f"--config {config_path}"]
proc = subprocess.run(cmd, capture_output=True, text=True)
print("stdout:", proc.stdout)
print("stderr:", proc.stderr)
proc.check_returncode()
