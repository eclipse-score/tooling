import subprocess
import os
import pytest
from pathlib import Path


def test_pyright():
    runfiles = os.getenv("RUNFILES_DIR")
    assert runfiles, "runfiles could not be found, RUNFILES_DIR is not set"
    packages = os.listdir(runfiles)
    #
    file_path = Path(os.path.realpath(__file__)).parent
    # Getting the python virtualenv
    python_venv_folder = [x for x in packages if "python_3_12_" in x][0]

    proc = subprocess.run(
        [
            python_venv_folder + "/bin/python",
            "-m",
            "basedpyright",
            "--project",
            file_path,
            "--warnings",
        ],
        cwd=runfiles,
        text=True,
        capture_output=True,
    )
    
    output = proc.stdout + proc.stderr
    if proc.returncode != 0:
        pytest.fail(output,False)
