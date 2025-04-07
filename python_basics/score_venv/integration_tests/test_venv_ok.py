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
from pathlib import Path
import os
import subprocess


def test_venv_ok(): 
    runfiles = os.getenv("RUNFILES_DIR")
    assert runfiles, "runfiles could not be found, RUNFILES_DIR is not set"
    packages = os.listdir(runfiles)
    assert any(x.endswith("pytest") for x in packages), f"'Pytest not found in runfiles: {runfiles}"
    try:
        import pytest # type ignore
        python_venv_folder = [x for x in packages if "python_3_12_" in x][0]

        # Trying to actually use pytest module and collect current test & file
        proc = subprocess.run(
            [python_venv_folder+"/bin/python", "-m", "pytest","--collect-only"],
            cwd=runfiles,
            check=True,
            capture_output=True
        )
        assert "test_venv_ok.py" in str(proc.stdout), "test_venv_ok.py, file not found in pytest collect"
        assert "test_venv_ok" in str(proc.stdout),  "test_venv_ok, test not found in pytest collect"
        assert proc.returncode == 0, f"Pytest collect didn't exit correctly: Exitcode: {proc.returncode}"


    except ImportError: 
        assert False, f"could not import pytest"
    except Exception as e: 
        assert False, f"something went wrong. Error: {e}"



