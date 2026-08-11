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

"""End-to-end / integration tests for the cr_checker tool.

Unlike `test_cr_checker.py` (unit tests calling individual functions), these
tests exercise the tool the way real consumers do:

* through the CLI entry point (`main`), not individual helper functions;
* against the *real* shipped `resources/templates.ini`, `config.json` and
  `exclusion.txt`, across every distinct comment style they define;
* against real files from this repository (dogfooding), and against a real
  (throwaway) git repository for the git-based file discovery path.
"""

from __future__ import annotations

import subprocess
import sys
from datetime import datetime
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))
from conftest import (  # noqa: E402
    PACKAGE_DIR,
    REAL_CONFIG_FILE,
    REAL_EXCLUSION_FILE,
    REAL_TEMPLATES_FILE,
    TOOL_MODULE_PATH,
    init_git_repo,
    load_cr_checker_module,
    load_template,
    write_config,
)

REPO_ROOT = PACKAGE_DIR.parent


# --- CLI smoke tests: exit code contract -----------------------------------


def test_main_returns_zero_when_all_files_compliant(tmp_path):
    cr_checker = load_cr_checker_module()
    config_file = write_config(tmp_path, "Author")
    header = load_template("py").format(year=datetime.now().year, author="Author")
    test_file = tmp_path / "compliant.py"
    test_file.write_text(header + "print('hi')\n", encoding="utf-8")

    exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])

    assert exit_code == 0


def test_main_returns_one_when_header_missing(tmp_path):
    cr_checker = load_cr_checker_module()
    config_file = write_config(tmp_path, "Author")
    test_file = tmp_path / "missing.py"
    test_file.write_text("print('hi')\n", encoding="utf-8")

    exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])

    assert exit_code == 1


def test_main_returns_two_on_unreadable_template(tmp_path):
    cr_checker = load_cr_checker_module()
    missing_template = tmp_path / "does_not_exist.ini"
    test_file = tmp_path / "file.py"
    test_file.write_text("print('hi')\n", encoding="utf-8")

    exit_code = cr_checker.main(["-t", str(missing_template), str(test_file)])

    assert exit_code == 2


# --- Different comment signs: fix + check cycle across every real style ----

# One representative extension per distinct comment style declared in the
# real templates.ini: "/* */" (cpp family), "#" (py family), "//" (rs),
# "'" (puml), "<!-- -->" (md) and the indented ".." (rst).
COMMENT_STYLE_SAMPLES = {
    "cpp": ("sample.cpp", "int main() { return 0; }\n", "/*"),
    "py": ("sample.py", "print('hello')\n", "#"),
    "rs": ("sample.rs", "fn main() {}\n", "//"),
    "puml": ("sample.puml", "@startuml\n@enduml\n", "'"),
    "md": ("sample.md", "# Heading\n", "<!--"),
    "rst": ("sample.rst", "Heading\n=======\n", ".."),
}


@pytest.mark.parametrize("extension", sorted(COMMENT_STYLE_SAMPLES))
def test_fix_then_check_across_comment_styles(tmp_path, extension):
    cr_checker = load_cr_checker_module()
    filename, body, marker = COMMENT_STYLE_SAMPLES[extension]
    config_file = write_config(tmp_path, "Author")
    test_file = tmp_path / filename
    test_file.write_text(body, encoding="utf-8")

    fix_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), "--fix", str(test_file)])
    check_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])

    fixed_content = test_file.read_text(encoding="utf-8")
    assert fix_exit_code == 1  # a --fix run still reports the pre-fix violation count
    assert check_exit_code == 0
    assert fixed_content.startswith(marker)
    # Exactly one blank line must separate the header from the original body,
    # never zero and never two.
    assert ("\n\n" + body) in fixed_content
    assert ("\n\n\n" + body) not in fixed_content


# --- Duplicate header detection, exercised through the CLI -----------------


@pytest.mark.parametrize("extension,filename", [("py", "dup.py"), ("cpp", "dup.cpp")])
def test_main_detects_duplicate_header_and_leaves_file_untouched(tmp_path, extension, filename):
    cr_checker = load_cr_checker_module()
    header_template = load_template(extension)
    config_file = write_config(tmp_path, "Author")
    header = header_template.format(year=datetime.now().year, author="Author")
    test_file = tmp_path / filename
    original_content = header + header + "body\n"
    test_file.write_text(original_content, encoding="utf-8")

    check_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])
    fix_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), "--fix", str(test_file)])

    assert check_exit_code == 1
    assert fix_exit_code == 1
    # Duplicate headers are reported, not silently "fixed" away.
    assert test_file.read_text(encoding="utf-8") == original_content


def test_main_detects_duplicate_header_with_different_year_ranges(tmp_path):
    """Two near-identical headers (same tool, same comment style, only the
    year differs) are highly similar to each other -- this is the "same
    header pasted twice" shape rapidfuzz should score high -- and must still
    be flagged as a duplicate and left completely untouched by --fix."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config_file = write_config(tmp_path, "Author")
    header1 = header_template.format(year="2024", author="Author")
    header2 = header_template.format(year="2024-2026", author="Author")
    test_file = tmp_path / "dup_years.py"
    original_content = header1 + header2 + "body\n"
    test_file.write_text(original_content, encoding="utf-8")

    check_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])
    fix_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), "--fix", str(test_file)])

    assert check_exit_code == 1
    assert fix_exit_code == 1
    assert test_file.read_text(encoding="utf-8") == original_content


def test_main_detects_cross_tool_duplicate_header_and_leaves_file_untouched(tmp_path):
    """A cr_checker-style header sitting next to a leftover REUSE-style
    header (e.g. from a partial migration) is structurally very different
    from it -- rapidfuzz should score this pair low -- but it must still be
    flagged as a duplicate: `has_duplicate_copyright`'s detection is
    intentionally format-agnostic, and similarity is only ever used to
    describe the finding, never to gate it. --fix must never try to merge
    or delete either header automatically."""
    cr_checker = load_cr_checker_module()
    config_file = write_config(tmp_path, "Author")
    header = load_template("py").format(year=datetime.now().year, author="Author")
    reuse_style_header = "# SPDX-FileCopyrightText: 2026 Author\n# SPDX-License-Identifier: Apache-2.0\n"
    test_file = tmp_path / "cross_tool_dup.py"
    original_content = header + reuse_style_header + "print('hi')\n"
    test_file.write_text(original_content, encoding="utf-8")

    check_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])
    fix_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), "--fix", str(test_file)])

    assert check_exit_code == 1
    assert fix_exit_code == 1
    assert test_file.read_text(encoding="utf-8") == original_content


# --- Exactly-one-blank-line-after-fix regression tests ----------------------


@pytest.mark.parametrize(
    "extension,filename,body",
    [
        ("py", "leading_blank.py", "\n\nprint('hi')\n"),
        ("cpp", "leading_blank.cpp", "\n\nint main() {}\n"),
    ],
)
def test_fix_normalizes_double_blank_line_to_single(tmp_path, extension, filename, body):
    """Regression test: a file whose body already starts with blank line(s)
    must not end up with a double blank line after the inserted header."""
    cr_checker = load_cr_checker_module()
    header_template = load_template(extension)
    test_file = tmp_path / filename
    test_file.write_text(body, encoding="utf-8")

    cr_checker.process_files([test_file], {extension: header_template}, True, use_mmap=False, encoding="utf-8")

    fixed = test_file.read_text(encoding="utf-8")
    stripped_body = body.lstrip("\n")
    assert fixed.endswith("\n\n" + stripped_body)
    assert not fixed.endswith("\n\n\n" + stripped_body)


def test_fix_adds_single_blank_line_when_none_present(tmp_path):
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    test_file = tmp_path / "no_blank.py"
    test_file.write_text("print('hi')\n", encoding="utf-8")

    cr_checker.process_files([test_file], {"py": header_template}, True, use_mmap=False, encoding="utf-8")

    assert test_file.read_text(encoding="utf-8").endswith("\n\nprint('hi')\n")


def test_fix_normalizes_blank_line_after_shebang(tmp_path):
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    test_file = tmp_path / "script.py"
    test_file.write_text("#!/usr/bin/env python3\n\nprint('hi')\n", encoding="utf-8")

    cr_checker.process_files([test_file], {"py": header_template}, True, use_mmap=False, encoding="utf-8")

    fixed = test_file.read_text(encoding="utf-8")
    assert fixed.startswith("#!/usr/bin/env python3\n")
    assert fixed.endswith("\n\nprint('hi')\n")
    assert "\n\n\n" not in fixed


def test_match_shebang_on_rust_script():
    """Rust supports shebang-based scripts (e.g. via the `rust-script`
    runner), even though the `.rs` template uses `//` comments rather than
    `#`. The shebang must still be detected and its char length returned."""
    cr_checker = load_cr_checker_module()
    shebang = "#!/usr/bin/env rust-script\n"

    consumed = cr_checker._match_shebang(shebang + "fn main() {}\n")

    assert consumed == len(shebang)


def test_match_shebang_ignores_rust_inner_attribute_in_rs_file():
    """Regression test: Rust's inner-attribute syntax `#![...]` (e.g.
    `#![cfg_attr(test, allow(dead_code))]`) also starts with `#!`, but is
    not a shebang and must not be mistaken for one -- doing so previously
    caused `--fix` to crash with "Invalid offset value" on real `.rs`
    files starting with such an attribute."""
    cr_checker = load_cr_checker_module()

    consumed = cr_checker._match_shebang("#![cfg_attr(test, allow(dead_code))]\n\nfn main() {}\n")

    assert consumed == 0


def test_main_fix_then_check_rust_file_with_shebang(tmp_path):
    """End-to-end: fixing a shebang'd `.rs` file must preserve the shebang as
    the very first line, insert the `//`-style header right after it, and
    leave exactly one blank line before the original body."""
    cr_checker = load_cr_checker_module()
    config_file = write_config(tmp_path, "Author")
    shebang = "#!/usr/bin/env rust-script\n"
    body = "fn main() {}\n"
    test_file = tmp_path / "script.rs"
    test_file.write_text(shebang + body, encoding="utf-8")

    fix_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), "--fix", str(test_file)])
    check_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])

    fixed_content = test_file.read_text(encoding="utf-8")
    assert fix_exit_code == 1  # a --fix run still reports the pre-fix violation count
    assert check_exit_code == 0
    assert fixed_content.startswith(shebang)
    assert fixed_content.startswith(shebang + "//")
    assert ("\n\n" + body) in fixed_content
    assert ("\n\n\n" + body) not in fixed_content


# --- rapidfuzz-gated auto-fix for wrong-format (but near-matching) headers -


def test_fix_reformats_near_matching_header_missing_angle_brackets(tmp_path):
    """Regression test for the real repo-wide drift found when auditing
    `.rs` files: a header that is otherwise identical to the template but
    is missing the `<>` around the license URL is a pure formatting slip,
    not a different license -- rapidfuzz should score it as a close match,
    so `--fix` safely strips and reformats it instead of just warning."""
    cr_checker = load_cr_checker_module()
    config_file = write_config(tmp_path, "Author")
    body = "fn main() {}\n"
    wrong_format_header = (
        "// *******************************************************************************\n"
        "// Copyright (c) 2024 Someone Else\n"
        "//\n"
        "// See the NOTICE file(s) distributed with this work for additional\n"
        "// information regarding copyright ownership.\n"
        "//\n"
        "// This program and the accompanying materials are made available under the\n"
        "// terms of the Apache License Version 2.0 which is available at\n"
        "// https://www.apache.org/licenses/LICENSE-2.0\n"
        "//\n"
        "// SPDX-License-Identifier: Apache-2.0\n"
        "// *******************************************************************************\n"
    )
    test_file = tmp_path / "drifted.rs"
    test_file.write_text(wrong_format_header + body, encoding="utf-8")

    check_before = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])
    fix_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), "--fix", str(test_file)])
    check_after = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])

    fixed_content = test_file.read_text(encoding="utf-8")
    assert check_before == 1  # wrong-format header is reported as a violation, even without --fix
    assert fix_exit_code == 1  # a --fix run that resolves a violation reports the pre-fix count
    assert check_after == 0  # the reformatted header now matches the template
    assert "<https://www.apache.org/licenses/LICENSE-2.0>" in fixed_content
    assert "Someone Else" not in fixed_content  # the old header text was replaced, not kept
    assert fixed_content.count("SPDX-License-Identifier") == 1  # no leftover duplicate
    assert fixed_content.endswith(body)


def test_fix_leaves_unrelated_license_text_untouched(tmp_path):
    """Safety test: a header that merely happens to contain the word
    'Copyright' and an (unrelated) SPDX identifier must NOT be auto-rewritten
    by `--fix`, since rapidfuzz should score it far below the template --
    only near-identical formatting drift is safe to auto-correct."""
    cr_checker = load_cr_checker_module()
    config_file = write_config(tmp_path, "Author")
    body = "fn main() {}\n"
    unrelated_header = (
        "// Copyright (c) 2020 Some Other Corp. All rights reserved.\n"
        "// Licensed under the MIT License; see the LICENSE file for details.\n"
        "//\n"
        "// SPDX-License-Identifier: MIT\n"
    )
    test_file = tmp_path / "unrelated.rs"
    test_file.write_text(unrelated_header + body, encoding="utf-8")

    fix_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), "--fix", str(test_file)])
    check_exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), str(test_file)])

    # Low similarity means `--fix` refuses to touch it -- so the file is left
    # untouched, but it's still reported as a violation both runs since it
    # never actually gets fixed.
    assert fix_exit_code == 1
    assert check_exit_code == 1
    assert test_file.read_text(encoding="utf-8") == unrelated_header + body


# --- Real use cases: dogfooding against this repository's actual files -----


def test_real_tool_source_file_is_compliant():
    """The tool's own source file must satisfy the real, shipped template."""
    cr_checker = load_cr_checker_module()
    templates = cr_checker.load_templates(REAL_TEMPLATES_FILE)

    results = cr_checker.process_files([TOOL_MODULE_PATH], templates, fix=False)

    assert results == {
        "missing": 0,
        "misplaced": 0,
        "wrong_format": 0,
        "duplicate": 0,
        "license_mismatch": 0,
        "fixed": 0,
    }


def test_real_build_file_is_compliant():
    """A real, extension-less BUILD file from this package must be compliant."""
    cr_checker = load_cr_checker_module()
    templates = cr_checker.load_templates(REAL_TEMPLATES_FILE)
    build_file = PACKAGE_DIR / "tool" / "BUILD"

    results = cr_checker.process_files([build_file], templates, fix=False)

    assert results == {
        "missing": 0,
        "misplaced": 0,
        "wrong_format": 0,
        "duplicate": 0,
        "license_mismatch": 0,
        "fixed": 0,
    }


def test_real_exclusion_file_skips_real_templates_ini():
    """`resources/templates.ini` cannot carry the '# Copyright ...' header
    itself (it defines that header), which is exactly why it's listed in the
    real, shipped exclusion.txt. Verify that combination actually works."""
    cr_checker = load_cr_checker_module()
    exclusion, valid = cr_checker.load_exclusion(REAL_EXCLUSION_FILE, str(REPO_ROOT))
    assert valid
    templates = cr_checker.load_templates(REAL_TEMPLATES_FILE)

    results = cr_checker.process_files(
        [REAL_TEMPLATES_FILE],
        templates,
        fix=False,
        exclusion=exclusion,
    )

    assert results == {
        "missing": 0,
        "misplaced": 0,
        "wrong_format": 0,
        "duplicate": 0,
        "license_mismatch": 0,
        "fixed": 0,
    }


def test_real_config_author_is_used_when_fixing(tmp_path):
    """Uses the real, shipped config.json (not a synthetic one) to fix a file."""
    cr_checker = load_cr_checker_module()
    templates = cr_checker.load_templates(REAL_TEMPLATES_FILE)
    real_author = cr_checker.get_author_from_config(REAL_CONFIG_FILE)
    test_file = tmp_path / "missing.py"
    test_file.write_text("print('hi')\n", encoding="utf-8")

    cr_checker.process_files([test_file], templates, fix=True, config=REAL_CONFIG_FILE)

    assert real_author in test_file.read_text(encoding="utf-8")


# --- Real use case: git-based discovery against the actual repository ------


def test_collect_inputs_against_real_repository():
    cr_checker = load_cr_checker_module()
    files = cr_checker.collect_inputs(["cr_checker/resources"], exts=None, base_dir=str(REPO_ROOT))

    names = {f.name for f in files}
    assert "templates.ini" in names
    assert "config.json" in names
    assert "exclusion.txt" in names


# --- git-based discovery (collect_inputs / list_tracked_files) -------------


def test_collect_inputs_honors_gitignore(tmp_path):
    cr_checker = load_cr_checker_module()
    init_git_repo(tmp_path)
    (tmp_path / ".gitignore").write_text("ignored.py\n", encoding="utf-8")
    (tmp_path / "visible.py").write_text("print('visible')\n", encoding="utf-8")
    (tmp_path / "ignored.py").write_text("print('ignored')\n", encoding="utf-8")

    files = cr_checker.collect_inputs([], exts=["py"], base_dir=str(tmp_path))

    names = {f.name for f in files}
    assert "visible.py" in names
    assert "ignored.py" not in names


def test_collect_inputs_matches_build_files_by_extension_filter(tmp_path):
    cr_checker = load_cr_checker_module()
    init_git_repo(tmp_path)
    (tmp_path / "BUILD").write_text("# a bazel build file\n", encoding="utf-8")
    (tmp_path / "other.txt").write_text("not matched\n", encoding="utf-8")

    files = cr_checker.collect_inputs([], exts=["BUILD"], base_dir=str(tmp_path))

    names = {f.name for f in files}
    assert "BUILD" in names
    assert "other.txt" not in names


# --- symlink handling --------------------------------------------------------


def test_collect_inputs_skips_tracked_symlink(tmp_path):
    """A symlink (even if tracked by git and pointing at a real file) must
    never be collected -- checking/fixing through it would mean the same
    underlying file gets processed twice whenever both the real path and a
    symlink to it are discoverable, and `--fix` writing through a symlink
    that points outside the intended tree is a real hazard."""
    cr_checker = load_cr_checker_module()
    init_git_repo(tmp_path)
    real_file = tmp_path / "real.py"
    real_file.write_text("print('hi')\n", encoding="utf-8")
    symlink = tmp_path / "link.py"
    symlink.symlink_to(real_file)
    _git(["add", "-A"], tmp_path)

    files = cr_checker.collect_inputs([], exts=["py"], base_dir=str(tmp_path))

    names = {f.name for f in files}
    assert "real.py" in names
    assert "link.py" not in names


def test_collect_inputs_skips_symlink_passed_explicitly(tmp_path):
    """The symlink skip applies even when the symlink path is passed
    directly as an input, not just when discovered via git."""
    cr_checker = load_cr_checker_module()
    init_git_repo(tmp_path)
    real_file = tmp_path / "real.py"
    real_file.write_text("print('hi')\n", encoding="utf-8")
    symlink = tmp_path / "link.py"
    symlink.symlink_to(real_file)
    _git(["add", "-A"], tmp_path)

    files = cr_checker.collect_inputs([str(symlink)], exts=["py"], base_dir=str(tmp_path))

    assert files == []


def test_main_modified_only_skips_modified_symlink(tmp_path, monkeypatch):
    """A symlink that shows up as "modified" (e.g. re-pointed, or newly
    added) must still be excluded by `--modified-only`, matching
    `collect_inputs`'s behavior for the non-incremental path."""
    cr_checker = load_cr_checker_module()
    init_git_repo(tmp_path)
    config_file = write_config(tmp_path, "Author")
    real_file = tmp_path / "real.py"
    real_file.write_text("print('hi')\n", encoding="utf-8")
    _git(["add", "-A"], tmp_path)
    _git(["commit", "-q", "-m", "baseline"], tmp_path)

    symlink = tmp_path / "link.py"
    symlink.symlink_to(real_file)
    _git(["add", "-A"], tmp_path)

    monkeypatch.setenv("BUILD_WORKSPACE_DIRECTORY", str(tmp_path))
    exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), "--modified-only"])

    # real.py has a header and was unmodified; link.py is new but must be
    # skipped entirely (not reported as missing a header, and never written
    # to), so nothing should be flagged.
    assert exit_code == 0
    assert symlink.is_symlink()  # untouched, never processed as a regular file


# --- --modified-only / list_modified_files ----------------------------------


def _git(args, cwd):
    subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True)


def test_list_modified_files_excludes_deleted_and_untouched_files(tmp_path):
    cr_checker = load_cr_checker_module()
    init_git_repo(tmp_path)

    (tmp_path / "untouched.py").write_text("print('untouched')\n", encoding="utf-8")
    (tmp_path / "modified.py").write_text("print('before')\n", encoding="utf-8")
    (tmp_path / "deleted.py").write_text("print('bye')\n", encoding="utf-8")
    _git(["add", "."], tmp_path)
    _git(["commit", "-q", "-m", "baseline"], tmp_path)

    (tmp_path / "modified.py").write_text("print('after')\n", encoding="utf-8")
    (tmp_path / "added.py").write_text("print('new')\n", encoding="utf-8")
    (tmp_path / "deleted.py").unlink()
    # `git diff HEAD` only notices a new file once it's staged (matches how
    # pre-commit itself only ever sees staged files) -- an untracked file
    # that was never `git add`ed is invisible to `git diff` entirely.
    _git(["add", "added.py"], tmp_path)

    files = cr_checker.list_modified_files(str(tmp_path))

    names = {f.name for f in files}
    assert names == {"modified.py", "added.py"}


def test_main_modified_only_ignores_untouched_noncompliant_files(tmp_path, monkeypatch):
    cr_checker = load_cr_checker_module()
    init_git_repo(tmp_path)
    config_file = write_config(tmp_path, "Author")

    header = load_template("py").format(year=datetime.now().year, author="Author")
    compliant = tmp_path / "compliant.py"
    compliant.write_text(header + "print('hi')\n", encoding="utf-8")
    # Committed without a header: --modified-only must leave it unreported
    # since it isn't part of this change.
    untouched_noncompliant = tmp_path / "untouched_noncompliant.py"
    untouched_noncompliant.write_text("print('no header')\n", encoding="utf-8")
    _git(["add", "."], tmp_path)
    _git(["commit", "-q", "-m", "baseline"], tmp_path)

    changed_noncompliant = tmp_path / "changed_noncompliant.py"
    changed_noncompliant.write_text("print('also no header')\n", encoding="utf-8")
    _git(["add", "changed_noncompliant.py"], tmp_path)

    monkeypatch.setenv("BUILD_WORKSPACE_DIRECTORY", str(tmp_path))
    exit_code = cr_checker.main(["-t", str(REAL_TEMPLATES_FILE), "-c", str(config_file), "--modified-only"])

    assert exit_code == 1  # only changed_noncompliant.py is reported
