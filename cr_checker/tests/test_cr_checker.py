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
import pytest
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
def load_template(extension: str) -> str:
    cr_checker = load_cr_checker_module()
    template_file = Path(__file__).resolve().parents[1] / "resources" / "templates.ini"
    templates = cr_checker.load_templates(template_file)
    return templates[extension]


# write the config file here so that the year is always up to date with the year
# written in the test file
def write_config(path: Path, author: str) -> Path:
    config_path = path / "config.json"
    config_path.write_text(json.dumps({"author": author}), encoding="utf-8")
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


@pytest.fixture(
    params=[
        "cpp",
        "c",
        "h",
        "hpp",
        "py",
        "sh",
        "bzl",
        "ini",
        "yml",
        "BUILD",
        "bazel",
        "rs",
        "rst",
    ]
)
def prepare_test_with_header(request: SubRequest, tmp_path: PosixPath) -> tuple:
    extension = request.param
    test_file = tmp_path / ("file." + extension)
    header_template = load_template(extension)
    current_year = datetime.now().year
    header = header_template.format(year=current_year, author="Author")
    test_file.write_text(
        header + "some content\n",
        encoding="utf-8",
    )
    return test_file, extension, header_template


@pytest.fixture(
    params=[
        "cpp",
        "c",
        "h",
        "hpp",
        "py",
        "sh",
        "bzl",
        "ini",
        "yml",
        "BUILD",
        "bazel",
        "rs",
        "rst",
    ]
)
def prepare_test_no_header(request: SubRequest, tmp_path: PosixPath) -> tuple:
    extension = request.param
    test_file = tmp_path / ("file." + extension)
    header_template = load_template(extension)
    current_year = datetime.now().year
    test_file.write_text(
        "some content\n",
        encoding="utf-8",
    )
    return test_file, extension, header_template, tmp_path


def test_process_files_detects_header(prepare_test_with_header):
    cr_checker = load_cr_checker_module()
    test_file, extension, header_template = prepare_test_with_header

    results = cr_checker.process_files(
        [test_file],
        {extension: header_template},
        False,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["no_copyright"] == 0


def test_process_files_detects_missing_header(prepare_test_no_header):
    cr_checker = load_cr_checker_module()
    test_file, extension, header_template, tmp_path = prepare_test_no_header

    results = cr_checker.process_files(
        [test_file],
        {extension: header_template},
        False,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["no_copyright"] == 1


def test_process_files_inserts_missing_header(prepare_test_no_header):
    cr_checker = load_cr_checker_module()
    test_file, extension, header_template, tmp_path = prepare_test_no_header
    author = "Author"
    config = write_config(tmp_path, author)

    results = cr_checker.process_files(
        [test_file],
        {extension: header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["no_copyright"] == 1
    assert results["fixed"] == 1
    expected_header = header_template.format(year=datetime.now().year, author="Author")
    assert test_file.read_text(encoding="utf-8").startswith(expected_header)


def test_process_files_skips_exclusion_with_missing_header(prepare_test_no_header):
    cr_checker = load_cr_checker_module()
    test_file, extension, header_template, tmp_path = prepare_test_no_header

    results = cr_checker.process_files(
        [test_file],
        {extension: header_template},
        False,
        [str(test_file)],
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["no_copyright"] == 0


# test that process_files function validates a license header after the shebang line
def test_process_files_accepts_header_after_shebang(tmp_path):
    cr_checker = load_cr_checker_module()
    script = tmp_path / "script.py"
    header_template = load_template("py")
    current_year = datetime.now().year
    header = header_template.format(year=current_year, author="Author")
    script.write_text(
        "#!/usr/bin/env python3\n" + header + "print('hi')\n",
        encoding="utf-8",
    )

    results = cr_checker.process_files(
        [script],
        {"py": header_template},
        False,
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
    header_template = load_template("py")
    current_year = datetime.now().year
    author = "Author"
    config = write_config(tmp_path, author)

    results = cr_checker.process_files(
        [script],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["fixed"] == 1
    assert results["no_copyright"] == 1
    expected_header = header_template.format(year=current_year, author=author)
    assert script.read_text(encoding="utf-8") == (
        "#!/usr/bin/env python3\n" + expected_header + "print('hi')\n"
    )


# test that process_files function validates a license header without the shebang line
def test_process_files_accepts_header_without_shebang(tmp_path):
    cr_checker = load_cr_checker_module()
    script = tmp_path / "script.py"
    header_template = load_template("py")
    current_year = datetime.now().year
    header = header_template.format(year=current_year, author="Author")
    script.write_text(header + "print('hi')\n", encoding="utf-8")

    results = cr_checker.process_files(
        [script],
        {"py": header_template},
        False,
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
    header_template = load_template("py")
    current_year = datetime.now().year
    author = "Author"
    config = write_config(tmp_path, author)

    results = cr_checker.process_files(
        [script],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["fixed"] == 1
    assert results["no_copyright"] == 1
    expected_header = header_template.format(year=current_year, author=author)
    assert script.read_text(encoding="utf-8") == expected_header + "print('hi')\n"
