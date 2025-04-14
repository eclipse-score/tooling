# from __future__ import annotations
#
# from basedpyright.run_node import run
#
#
# def main():
#     run("index")
#
# if __name__ == "__main__":
#     main()

import subprocess
import os
import pytest
import sys
from pathlib import Path

def test_pyright():
    runfiles = os.getenv("RUNFILES_DIR")
    assert runfiles, "runfiles could not be found, RUNFILES_DIR is not set"
    packages = os.listdir(runfiles)
    #
    # The target project path is now passed as a command-line argument
    # Getting the python virtualenv
    python_venv_folder = [x for x in packages if "python_3_12_" in x][0]
    print(f"RUNNING IN: {runfiles}")
    proc = subprocess.run(
        [
            f"{runfiles}/{python_venv_folder}/bin/python",
            "-m",
            "basedpyright",
            "--warnings",
        ],
        cwd=runfiles,
        text=True,
        capture_output=True,
    )
    output = proc.stdout + proc.stderr
    if proc.returncode != 0:
        pytest.fail(output, False)
