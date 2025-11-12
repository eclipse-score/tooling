# *******************************************************************************
# Copyright (c) 2024 Contributors to the Eclipse Foundation
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
# unit tests for the shebang handling in the cr_checker module
from __future__ import annotations

import importlib.util
import json
from datetime import datetime
from pathlib import Path


# load the cr_checker module
def load_cr_checker_module():
    module_path = Path(__file__).resolve().parents[1] / "tool" / "cr_checker.py"
    spec = importlib.util.spec_from_file_location("cr_checker_module", module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Failed to load cr_checker module from {module_path}")

    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# load the license template
def load_py_template() -> str:
    cr_checker = load_cr_checker_module()
    template_file = Path(__file__).resolve().parents[1] / "resources" / "templates.ini"
    templates = cr_checker.load_templates(template_file)
    return templates["py"]


# write the config file here so that the year is always up to date with the year
# written in the mock "script.py" file
def write_config(path: Path, years: list[int]) -> Path:
    config_path = path / "config.json"
    config_path.write_text(json.dumps({"years": years}), encoding="utf-8")
    return config_path


# test that offset matches the length of the shebang line including trailing newlines
def test_detect_shebang_offset_counts_trailing_newlines(tmp_path):
    cr_checker = load_cr_checker_module()
    script = tmp_path / "script.py"
    script.write_text(
        "#!/usr/bin/env python3\n\nprint('hi')\n",
        encoding="utf-8",
    )

    offset = cr_checker.detect_shebang_offset(script, "utf-8")

    assert offset == len("#!/usr/bin/env python3\n\n".encode("utf-8"))


# test that process_files function validates a license header after the shebang line
def test_process_files_accepts_header_after_shebang(tmp_path):
    cr_checker = load_cr_checker_module()
    script = tmp_path / "script.py"
    header_template = load_py_template()
    current_year = datetime.now().year
    header = header_template.format(year=current_year)
    script.write_text(
        "#!/usr/bin/env python3\n" + header + "print('hi')\n",
        encoding="utf-8",
    )
    config = write_config(tmp_path, [current_year])

    results = cr_checker.process_files(
        [script],
        {"py": header_template},
        False,
        config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["no_copyright"] == 0


# test that process_files function fixes a missing license header after the shebang line
def test_process_files_fix_inserts_header_after_shebang(tmp_path):
    cr_checker = load_cr_checker_module()
    script = tmp_path / "script.py"
    script.write_text(
        "#!/usr/bin/env python3\nprint('hi')\n",
        encoding="utf-8",
    )
    header_template = load_py_template()
    current_year = datetime.now().year
    config = write_config(tmp_path, [current_year])

    results = cr_checker.process_files(
        [script],
        {"py": header_template},
        True,
        config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["fixed"] == 1
    assert results["no_copyright"] == 1
    expected_header = header_template.format(year=current_year)
    assert script.read_text(encoding="utf-8") == (
        "#!/usr/bin/env python3\n" + expected_header + "print('hi')\n"
    )


# test that process_files function validates a license header without the shebang line
def test_process_files_accepts_header_without_shebang(tmp_path):
    cr_checker = load_cr_checker_module()
    script = tmp_path / "script.py"
    header_template = load_py_template()
    current_year = datetime.now().year
    header = header_template.format(year=current_year)
    script.write_text(header + "print('hi')\n", encoding="utf-8")
    config = write_config(tmp_path, [current_year])

    results = cr_checker.process_files(
        [script],
        {"py": header_template},
        False,
        config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["no_copyright"] == 0


# test that process_files function fixes a missing license header without the shebang
def test_process_files_fix_inserts_header_without_shebang(tmp_path):
    cr_checker = load_cr_checker_module()
    script = tmp_path / "script.py"
    script.write_text("print('hi')\n", encoding="utf-8")
    header_template = load_py_template()
    current_year = datetime.now().year
    config = write_config(tmp_path, [current_year])

    results = cr_checker.process_files(
        [script],
        {"py": header_template},
        True,
        config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["fixed"] == 1
    assert results["no_copyright"] == 1
    expected_header = header_template.format(year=current_year)
    assert script.read_text(encoding="utf-8") == expected_header + "print('hi')\n"
