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
import os
import subprocess
import sys
from pathlib import Path


def test_venv_ok():
    runfiles = os.getenv("RUNFILES_DIR")
    assert runfiles, "runfiles could not be found, RUNFILES_DIR is not set"
    packages = os.listdir(runfiles)
    assert any(x.endswith("pytest") for x in packages), (
        f"'Pytest not found in runfiles: {runfiles}"
    )
    try:
        import pytest  # type ignore

        python_venv_folder = [x for x in packages if "python_3_12_" in x][0]

        # Since rules_python 1.7.0, PYTHONPATH is no longer populated for
        # --bootstrap_impl=system_python, so the raw toolchain interpreter
        # invoked below no longer inherits the import paths this process
        # was started with. Rebuild PYTHONPATH from our own sys.path (which
        # is known-good, since `import pytest` above already succeeded) and
        # pass it explicitly so the subprocess can find pytest too.
        env = os.environ | {"PYTHONPATH": os.pathsep.join(filter(None, sys.path))}

        # Trying to actually use pytest module and collect current test & file.
        # Scope collection to "_main" (this workspace's own sources, canonical
        # repo name for the root module under bazel) instead of the whole
        # runfiles tree: sibling dirs there are third-party pip site-packages
        # (e.g. pylint's "dill" dependency ships its own dill/tests/test_*.py),
        # which pytest would otherwise also try to collect and fail to import.
        proc = subprocess.run(
            [
                python_venv_folder + "/bin/python",
                "-m",
                "pytest",
                "--collect-only",
                "_main",
            ],
            cwd=runfiles,
            check=True,
            capture_output=True,
            env=env,
        )
        assert "test_venv_ok.py" in str(proc.stdout), (
            "test_venv_ok.py, file not found in pytest collect"
        )
        assert "test_venv_ok" in str(proc.stdout), (
            "test_venv_ok, test not found in pytest collect"
        )
        assert proc.returncode == 0, (
            f"Pytest collect didn't exit correctly: Exitcode: {proc.returncode}"
        )

    except ImportError:
        assert False, f"could not import pytest"
    except Exception as e:
        assert False, f"something went wrong. Error: {e}"
