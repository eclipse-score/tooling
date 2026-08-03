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

import sys
from datetime import datetime
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))
from conftest import load_cr_checker_module, load_template, write_config  # noqa: E402


# test that _match_shebang consumes the shebang line but not trailing newlines
# (those are handled separately by `_match_prefix`'s `_skip_blank_lines` call)
def test_match_shebang_consumes_shebang_line():
    cr_checker = load_cr_checker_module()

    consumed = cr_checker._match_shebang("#!/usr/bin/env python3\n\nprint('hi')\n")

    assert consumed == len("#!/usr/bin/env python3\n")


# test that _match_shebang ignores Rust's `#![...]` inner-attribute syntax
def test_match_shebang_ignores_rust_inner_attribute():
    cr_checker = load_cr_checker_module()

    consumed = cr_checker._match_shebang("#![cfg_attr(not(test), no_std)]\nfn main() {}\n")

    assert consumed == 0


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
        "yaml",
        "BUILD",
        "bazel",
        "rs",
        "rst",
        "md",
        "puml",
    ]
)
def prepare_test_with_header(request, tmp_path):
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
        "yaml",
        "BUILD",
        "bazel",
        "rs",
        "rst",
        "md",
        "puml",
    ]
)
def prepare_test_no_header(request, tmp_path):
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

    assert results["missing"] == 0
    assert results["misplaced"] == 0
    assert results["wrong_format"] == 0
    assert results["duplicate"] == 0


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

    assert results["missing"] == 1


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

    assert results["missing"] == 1
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
        [str(test_file.resolve())],
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["missing"] == 0


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

    assert results["missing"] == 0
    assert results["misplaced"] == 0
    assert results["wrong_format"] == 0


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
    assert results["missing"] == 1
    expected_header = header_template.format(year=current_year, author=author)
    assert script.read_text(encoding="utf-8") == ("#!/usr/bin/env python3\n" + expected_header + "\n" + "print('hi')\n")


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

    assert results["missing"] == 0


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
    assert results["missing"] == 1
    expected_header = header_template.format(year=current_year, author=author)
    assert script.read_text(encoding="utf-8") == expected_header + "\n" + "print('hi')\n"


# test that border lines with different fill characters are accepted (flexible matching)
def test_process_files_accepts_flexible_border(tmp_path):
    cr_checker = load_cr_checker_module()
    test_file = tmp_path / "file.cpp"
    current_year = datetime.now().year
    # Use '/' fill chars instead of '*' for border lines
    header = (
        "/////////////////////////////////////////////////////////////////////////////////////\n"
        f" * Copyright (c) {current_year} Author\n"
        " *\n"
        " * See the NOTICE file(s) distributed with this work for additional\n"
        " * information regarding copyright ownership.\n"
        " *\n"
        " * This program and the accompanying materials are made available under the\n"
        " * terms of the Apache License Version 2.0 which is available at\n"
        " * https://www.apache.org/licenses/LICENSE-2.0\n"
        " *\n"
        " * SPDX-License-Identifier: Apache-2.0\n"
        " /////////////////////////////////////////////////////////////////////////////////////\n"
    )
    test_file.write_text(header + "int main() {}\n", encoding="utf-8")
    header_template = load_template("cpp")

    results = cr_checker.process_files(
        [test_file],
        {"cpp": header_template},
        False,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["missing"] == 0


# test that a blank line after the header does not cause a check failure
def test_process_files_accepts_header_with_trailing_blank_line(tmp_path):
    cr_checker = load_cr_checker_module()
    test_file = tmp_path / "file.py"
    header_template = load_template("py")
    current_year = datetime.now().year
    header = header_template.format(year=current_year, author="Author")
    test_file.write_text(header + "\nsome content\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        False,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["missing"] == 0


# test that fixing a missing header inserts a blank line after the header
def test_process_files_fix_inserts_trailing_blank_line(tmp_path):
    cr_checker = load_cr_checker_module()
    test_file = tmp_path / "file.py"
    test_file.write_text("some content\n", encoding="utf-8")
    header_template = load_template("py")
    current_year = datetime.now().year
    author = "Author"
    config = write_config(tmp_path, author)

    cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    expected_header = header_template.format(year=current_year, author=author)
    assert test_file.read_text(encoding="utf-8").startswith(expected_header + "\n")


# --- locate_header / classify (status classification) ---


def test_classify_detects_duplicate(tmp_path):
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    current_year = datetime.now().year
    header = header_template.format(year=current_year, author="Author")
    content = header + header + "some content\n"

    layout = cr_checker.locate_header(content)
    status, _ = cr_checker.classify(layout, header_template, None)

    assert status is cr_checker.Status.DUPLICATE
    assert len(layout.blocks) == 2


def test_classify_accepts_single_header(tmp_path):
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    current_year = datetime.now().year
    header = header_template.format(year=current_year, author="Author")
    content = header + "some content\n"

    layout = cr_checker.locate_header(content)
    status, _ = cr_checker.classify(layout, header_template, None)

    assert status is cr_checker.Status.COMPLIANT


def test_process_files_detects_duplicate_header(tmp_path):
    cr_checker = load_cr_checker_module()
    test_file = tmp_path / "file.py"
    header_template = load_template("py")
    current_year = datetime.now().year
    header = header_template.format(year=current_year, author="Author")
    test_file.write_text(header + header + "some content\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        False,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["duplicate"] == 1
    assert results["missing"] == 0


def test_classify_detects_duplicate_with_different_year_ranges(tmp_path):
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header1 = header_template.format(year="2026", author="Author")
    header2 = header_template.format(year="2024-2026", author="Author")
    content = header1 + header2 + "some content\n"

    layout = cr_checker.locate_header(content)
    status, _ = cr_checker.classify(layout, header_template, None)

    assert status is cr_checker.Status.DUPLICATE


# --- locate_header (block span) ---


def test_locate_header_block_covers_whole_block(tmp_path):
    """The detected block must include the border lines around the legal text
    and the trailing blank line, not just the "Copyright...SPDX" text itself."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=2024, author="Author")
    body = "print('hi')\n"
    content = header + body

    layout = cr_checker.locate_header(content)

    assert len(layout.blocks) == 1
    block = layout.blocks[0]
    assert content[block.start : block.end] == header
    assert content[block.end :] == body


def test_locate_header_returns_no_blocks_without_copyright(tmp_path):
    cr_checker = load_cr_checker_module()
    content = "print('hi')\n"

    layout = cr_checker.locate_header(content)

    assert layout.blocks == []


# --- classify (rapidfuzz-gated auto-fix similarity) ---


def test_classify_scores_formatting_drift_highly(tmp_path):
    """A header that differs from the template only by a cosmetic detail
    (missing angle brackets around the URL) must score well above the
    auto-fix threshold, since it's the same statement, just miswritten."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("rs")
    config_file = write_config(tmp_path, "Author")
    drifted_header = header_template.format(year=2024, author="Author").replace(
        "<https://www.apache.org/licenses/LICENSE-2.0>", "https://www.apache.org/licenses/LICENSE-2.0"
    )

    layout = cr_checker.locate_header(drifted_header)
    status, similarity = cr_checker.classify(layout, header_template, config_file)

    assert len(layout.blocks) == 1
    assert status is cr_checker.Status.WRONG_FORMAT
    assert similarity >= cr_checker.HEADER_SIMILARITY_THRESHOLD


def test_classify_scores_unrelated_license_low(tmp_path):
    """A header for a genuinely different license must score well below the
    auto-fix threshold, AND be caught by the SPDX mismatch guard -- either
    would prevent `--fix` from silently overwriting it."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("rs")
    config_file = write_config(tmp_path, "Author")
    unrelated_header = (
        "// Copyright (c) 2020 Some Other Corp. All rights reserved.\n"
        "// Licensed under the MIT License; see the LICENSE file for details.\n"
        "//\n"
        "// SPDX-License-Identifier: MIT\n"
    )

    layout = cr_checker.locate_header(unrelated_header)
    status, similarity = cr_checker.classify(layout, header_template, config_file)

    assert status is cr_checker.Status.LICENSE_MISMATCH
    assert similarity < cr_checker.HEADER_SIMILARITY_THRESHOLD


def test_spdx_mismatch_ignores_spacing_dots_and_hyphens(tmp_path):
    """A harmless formatting variant of the SAME identifier (spacing/dots/
    hyphens differ but the license doesn't) must NOT trip the mismatch guard."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config_file = write_config(tmp_path, "Author")
    variant_header = "# Copyright (c) 2024 Author\n#\n# SPDX-License-Identifier: apache 2.0\n"

    layout = cr_checker.locate_header(variant_header)
    status, _ = cr_checker.classify(layout, header_template, config_file)

    assert status is not cr_checker.Status.LICENSE_MISMATCH


# --- duplicate_similarity (diagnostics for DUPLICATE status) ---


def test_duplicate_similarity_scores_pasted_twice_highly(tmp_path):
    """The same header pasted twice (e.g. from running --fix twice) must
    score well above the threshold, signalling it's safe to just delete the
    extra copy."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=2024, author="Author")
    content = header + header + "print('hi')\n"

    layout = cr_checker.locate_header(content)
    similarity = cr_checker.duplicate_similarity(layout.blocks)

    assert similarity is not None
    assert similarity >= cr_checker.HEADER_SIMILARITY_THRESHOLD


def test_duplicate_similarity_scores_cross_tool_headers_low(tmp_path):
    """A cr_checker-style header sitting next to a leftover REUSE-style
    header (structurally very different, but both matched as blocks) must
    score low, signalling a manual merge is needed rather than a blind
    deletion."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=2024, author="Author")
    reuse_style_header = "# SPDX-FileCopyrightText: 2024 Author\n# SPDX-License-Identifier: Apache-2.0\n"
    content = header + reuse_style_header + "print('hi')\n"

    layout = cr_checker.locate_header(content)
    similarity = cr_checker.duplicate_similarity(layout.blocks)

    assert similarity is not None
    assert similarity < cr_checker.HEADER_SIMILARITY_THRESHOLD


def test_duplicate_similarity_returns_none_for_single_header(tmp_path):
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    content = header_template.format(year=2024, author="Author") + "print('hi')\n"

    layout = cr_checker.locate_header(content)

    assert cr_checker.duplicate_similarity(layout.blocks) is None


# --- MISPLACED (Option A: junk-based) classification ---


def test_classify_detects_misplaced_correct_header(tmp_path):
    """A compliant header preceded by real, unrecognized content (not a
    registered preamble like a shebang) is MISPLACED, not COMPLIANT."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=datetime.now().year, author="Author")
    content = "import os\n\n" + header + "print('hi')\n"

    layout = cr_checker.locate_header(content)
    status, _ = cr_checker.classify(layout, header_template, None)

    assert status is cr_checker.Status.MISPLACED
    assert layout.leading_junk == "import os\n\n"


def test_classify_detects_misplaced_and_wrong_format_header(tmp_path):
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config_file = write_config(tmp_path, "Author")
    drifted_header = header_template.format(year=2024, author="Author").replace("Copyright (c)", "Copyright")
    content = "import os\n\n" + drifted_header + "print('hi')\n"

    layout = cr_checker.locate_header(content)
    status, _ = cr_checker.classify(layout, header_template, config_file)

    assert status is cr_checker.Status.MISPLACED_AND_WRONG_FORMAT


def test_process_files_fix_relocates_misplaced_header(tmp_path):
    """`--fix` moves a correctly-formatted-but-misplaced header to the very
    top of the file (after any recognized preamble), preserving the
    unrecognized leading content immediately after it."""
    cr_checker = load_cr_checker_module()
    test_file = tmp_path / "file.py"
    header_template = load_template("py")
    author = "Author"
    config = write_config(tmp_path, author)
    header = header_template.format(year=datetime.now().year, author=author)
    test_file.write_text("import os\n\n" + header + "print('hi')\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["misplaced"] == 1
    assert results["fixed"] == 1
    assert test_file.read_text(encoding="utf-8") == header + "\n" + "import os\n\n" + "print('hi')\n"


def test_classify_shebang_preamble_is_not_misplaced(tmp_path):
    """A shebang is a recognized preamble (see `PREFIX_MATCHERS`), so a
    compliant header right after it is COMPLIANT, not MISPLACED."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=datetime.now().year, author="Author")
    content = "#!/usr/bin/env python3\n" + header + "print('hi')\n"

    layout = cr_checker.locate_header(content)
    status, _ = cr_checker.classify(layout, header_template, None)

    assert status is cr_checker.Status.COMPLIANT
    assert layout.leading_junk == ""


# --- COPYRIGHT_BLOCK_PATTERN precision (avoid false-positive "blocks") -----


def test_copyright_block_pattern_requires_colon_after_spdx_identifier():
    """A plain-English mention of both keywords (no ``: <license>`` tag) must
    not be mistaken for an actual SPDX header block -- otherwise a comment
    merely *describing* copyright/SPDX handling (e.g. in this tool's own
    source, or a docstring) is treated as a real header."""
    cr_checker = load_cr_checker_module()
    text = "# Copyright note: see SPDX-License-Identifier handling below\nbody\n"

    assert cr_checker.COPYRIGHT_BLOCK_PATTERN.search(text) is None
    layout = cr_checker.locate_header(text)
    assert layout.blocks == []


def test_copyright_block_pattern_rejects_gap_beyond_max():
    """ "Copyright" and "SPDX-License-Identifier: ..." separated by more than
    `COPYRIGHT_BLOCK_MAX_GAP` characters of unrelated content must not be
    stitched together into a single block -- that gap is far larger than any
    real template, so it can only be two unrelated mentions."""
    cr_checker = load_cr_checker_module()
    filler = "x" * (cr_checker.COPYRIGHT_BLOCK_MAX_GAP + 50)
    text = f"Copyright (c) 2024 Author\n{filler}\nSPDX-License-Identifier: Apache-2.0\n"

    assert cr_checker.COPYRIGHT_BLOCK_PATTERN.search(text) is None


def test_copyright_block_pattern_matches_within_max_gap():
    """A real template's gap (well under `COPYRIGHT_BLOCK_MAX_GAP`) must
    still match -- the tightened pattern isn't so strict it breaks real
    headers."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=2024, author="Author")

    assert cr_checker.COPYRIGHT_BLOCK_PATTERN.search(header) is not None


# --- normalize_header / classify duplicate-window consistency (data loss) --


def test_fix_preserves_body_with_coincidental_deep_mention(tmp_path):
    """Regression test: a fixable (wrong-format, high-similarity) header at
    the top of the file, followed far later by a line that merely mentions
    "Copyright" and "SPDX-License-Identifier" in prose (no real second
    header), must not cause `--fix` to delete everything in between. This
    previously happened because `_find_blocks` matched the deep mention as a
    second "block", and `normalize_header` deleted through the *last* block
    found anywhere in the file instead of only the in-window one(s)
    `classify` actually used."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    # Same statement, missing "(c)" -- a formatting drift, not a different
    # license, so it scores above HEADER_SIMILARITY_THRESHOLD and is fixable.
    drifted_header = header_template.format(year=2024, author="Author").replace("Copyright (c)", "Copyright")
    filler = "x = 1\n" * 200
    deep_mention = "# Copyright note: see SPDX-License-Identifier handling below\n"
    test_file = tmp_path / "file.py"
    test_file.write_text(drifted_header + filler + deep_mention + "y = 2\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    fixed = test_file.read_text(encoding="utf-8")
    assert results["fixed"] == 1
    assert "x = 1\n" * 200 in fixed
    assert deep_mention in fixed
    assert fixed.endswith("y = 2\n")


def test_normalize_header_leaves_out_of_window_block_untouched(tmp_path):
    """Direct unit test of the fix: `normalize_header` must only ever delete
    through the last block within the duplicate-detection window, never a
    block further into the file -- even if `_find_blocks` did somehow match
    one (e.g. a genuine second, unrelated SPDX tag far past the real
    header)."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=2024, author="Author").replace("Copyright (c)", "Copyright")
    filler = "x = 1\n" * 200
    far_header = "# Copyright (c) 2020 Example Corp\n# SPDX-License-Identifier: MIT\n"
    content = header + filler + far_header + "y = 2\n"

    layout = cr_checker.locate_header(content)
    assert len(layout.blocks) == 2  # the far, genuinely-SPDX-shaped block is still detected...

    new_text = cr_checker.normalize_header(content, layout, header_template, None)

    # ...but must survive verbatim in the rewritten output, since it's
    # outside the duplicate-detection window `classify` used to decide this
    # was a single fixable header, not the header itself.
    assert filler in new_text
    assert far_header in new_text
    assert new_text.endswith("y = 2\n")


# --- --remove-offset ---------------------------------------------------------


def test_process_files_fix_removes_offset_before_inserting_header(tmp_path):
    """`remove_offset` strips the given number of characters from the very
    start of the file (e.g. an old, invalid header) before the new header is
    inserted -- the removed text must not reappear anywhere in the result."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    old_header = "# OLD INVALID HEADER\n# more junk\n"
    body = "print('hi')\n"
    test_file = tmp_path / "file.py"
    test_file.write_text(old_header + body, encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=len(old_header),
    )

    fixed = test_file.read_text(encoding="utf-8")
    expected_header = header_template.format(year=datetime.now().year, author="Author")
    assert results["fixed"] == 1
    assert "OLD INVALID HEADER" not in fixed
    assert fixed == expected_header + "\n" + body


def test_process_files_check_mode_ignores_remove_offset(tmp_path):
    """`remove_offset` only applies in `--fix` mode (see `process_files`'s
    docstring); a check-only run must not be affected by it at all."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=datetime.now().year, author="Author")
    test_file = tmp_path / "file.py"
    test_file.write_text(header + "print('hi')\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        False,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=5,
    )

    assert results["missing"] == 0
    assert test_file.read_text(encoding="utf-8") == header + "print('hi')\n"


# --- --force (bypass the similarity guard) -----------------------------------


def test_process_files_fix_without_force_leaves_low_similarity_header_untouched(tmp_path):
    """Baseline: a WRONG_FORMAT header that scores below
    `HEADER_SIMILARITY_THRESHOLD` (unrecognizable boilerplate, though it
    carries the same SPDX identifier so it isn't a LICENSE_MISMATCH) must be
    left alone by `--fix` without `--force`."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    unrelated_header = (
        "# Copyright presence only, then completely different padding text follows\n"
        "# padding padding padding padding padding padding padding\n"
        "#\n"
        "# SPDX-License-Identifier: Apache-2.0\n"
    )
    test_file = tmp_path / "file.py"
    test_file.write_text(unrelated_header + "print('hi')\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
    )

    assert results["fixed"] == 0
    assert results["wrong_format"] == 1
    assert test_file.read_text(encoding="utf-8") == unrelated_header + "print('hi')\n"


def test_process_files_fix_force_rewrites_low_similarity_header(tmp_path):
    """With `force=True`, the same low-similarity (but same-license) header IS
    rewritten -- `--force` is an explicit, opt-in override of the similarity
    guard, though never of the separate SPDX mismatch guard (see
    `test_process_files_fix_force_does_not_touch_license_mismatch`)."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    unrelated_header = (
        "# Copyright presence only, then completely different padding text follows\n"
        "# padding padding padding padding padding padding padding\n"
        "#\n"
        "# SPDX-License-Identifier: Apache-2.0\n"
    )
    test_file = tmp_path / "file.py"
    test_file.write_text(unrelated_header + "print('hi')\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        force=True,
    )

    expected_header = header_template.format(year=datetime.now().year, author="Author")
    fixed = test_file.read_text(encoding="utf-8")
    assert results["fixed"] == 1
    assert "padding" not in fixed
    assert fixed == expected_header + "\n" + "print('hi')\n"


def test_process_files_fix_force_does_not_touch_license_mismatch(tmp_path):
    """`force` only bypasses the *similarity* guard for WRONG_FORMAT /
    MISPLACED_AND_WRONG_FORMAT; LICENSE_MISMATCH is not in `FIXABLE_STATUSES`
    at all and must still be left for manual review even with `force=True`,
    since silently overwriting a genuinely different license's SPDX
    identifier is a legal/compliance-significant action, not cosmetic
    drift."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    unrelated_header = (
        "# Copyright (c) 2020 Some Other Corp. All rights reserved.\n"
        "# Licensed under the MIT License; see the LICENSE file for details.\n"
        "#\n"
        "# SPDX-License-Identifier: MIT\n"
    )
    test_file = tmp_path / "file.py"
    test_file.write_text(unrelated_header + "print('hi')\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        force=True,
    )

    assert results["fixed"] == 0
    assert results["license_mismatch"] == 1
    assert test_file.read_text(encoding="utf-8") == unrelated_header + "print('hi')\n"


def test_process_files_fix_force_does_not_touch_duplicate(tmp_path):
    """`force` only bypasses the *similarity* guard for WRONG_FORMAT /
    MISPLACED_AND_WRONG_FORMAT; DUPLICATE is not in `FIXABLE_STATUSES` at
    all and must still be left for manual review even with `force=True`."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    header = header_template.format(year=2024, author="Author")
    content = header + header + "print('hi')\n"
    test_file = tmp_path / "file.py"
    test_file.write_text(content, encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        force=True,
    )

    assert results["fixed"] == 0
    assert results["duplicate"] == 1
    assert test_file.read_text(encoding="utf-8") == content


def test_process_files_check_mode_ignores_force(tmp_path):
    """`force` only applies in `--fix` mode; a check-only run must not be
    affected by it at all (no similarity gate exists in check mode to begin
    with)."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    unrelated_header = (
        "# Copyright (c) 2020 Some Other Corp. All rights reserved.\n"
        "# Licensed under the MIT License; see the LICENSE file for details.\n"
        "#\n"
        "# SPDX-License-Identifier: MIT\n"
    )
    test_file = tmp_path / "file.py"
    test_file.write_text(unrelated_header + "print('hi')\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        False,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        force=True,
    )

    assert results["fixed"] == 0
    assert results["license_mismatch"] == 1
    assert test_file.read_text(encoding="utf-8") == unrelated_header + "print('hi')\n"


# --- old-style comment-wrapper debris (regression: orphaned <!--/-->/.. ) ----


def test_is_old_wrapper_prefix_recognizes_bare_html_comment_opener():
    """A bare `<!--` line -- as used by an older, since-superseded md header
    style -- is a prefix of the current md template's first line, so it must
    be recognized as old-header debris, not genuine misplaced content."""
    cr_checker = load_cr_checker_module()
    md_template = load_template("md")

    assert cr_checker._is_old_wrapper_prefix("<!--\n", md_template) is True


def test_is_old_wrapper_prefix_recognizes_bare_rst_comment_marker():
    """A bare `..` line -- the RST comment marker with no border of its own
    -- is (trivially) a prefix of the rst template's first line (which is
    exactly `..`), so it must be recognized as old-header debris."""
    cr_checker = load_cr_checker_module()
    rst_template = load_template("rst")

    assert cr_checker._is_old_wrapper_prefix("..\n", rst_template) is True


def test_is_old_wrapper_prefix_rejects_genuine_preceding_content():
    """Real, unrelated content sitting before the header (a genuine MISPLACED
    case) must NOT be mistaken for wrapper debris, even if it happens to be
    short."""
    cr_checker = load_cr_checker_module()
    py_template = load_template("py")

    assert cr_checker._is_old_wrapper_prefix("import os\n", py_template) is False


def test_strip_old_wrapper_suffix_drops_bare_html_comment_closer():
    """A bare `-->` line left over from an older md header (whose closing
    marker wasn't folded onto the border, unlike the current template) must
    be dropped from the remainder, not preserved as orphaned content."""
    cr_checker = load_cr_checker_module()
    md_template = load_template("md")

    remainder = cr_checker._strip_old_wrapper_suffix("-->\n\n# Heading\n", md_template)

    assert remainder == "# Heading\n"


def test_strip_old_wrapper_suffix_preserves_genuine_content():
    """Real content right after the header must be left alone."""
    cr_checker = load_cr_checker_module()
    md_template = load_template("md")

    remainder = cr_checker._strip_old_wrapper_suffix("# Heading\n", md_template)

    assert remainder == "# Heading\n"


def test_process_files_fix_force_rewrites_old_style_md_header_cleanly(tmp_path):
    """Regression test for a real bug: a `.md` file with an OLDER header
    style -- content wrapped in a single ``<!-- ... -->`` HTML comment with
    an asterisk border, rather than the current template's dash-border-with-
    inline-``<!--``/``-->`` style -- must, after a forced `--fix`, end up
    with exactly ONE clean header and no orphaned ``<!--``/``-->`` fragment
    left over from the old wrapper (previously, the leftover bare ``<!--``
    was blindly re-appended after the new header, and the leftover bare
    ``-->`` leaked into the body untouched, producing a dangling empty
    HTML comment right after the real one)."""
    cr_checker = load_cr_checker_module()
    md_template = load_template("md")
    config = write_config(tmp_path, "Author")
    old_style_header = (
        "<!--\n"
        "*******************************************************************************\n"
        "Copyright (c) 2026 Contributors to the Eclipse Foundation\n"
        "\n"
        "See the NOTICE file(s) distributed with this work for additional\n"
        "information regarding copyright ownership.\n"
        "\n"
        "This program and the accompanying materials are made available under the\n"
        "terms of the Apache License Version 2.0 which is available at\n"
        "https://www.apache.org/licenses/LICENSE-2.0\n"
        "\n"
        "SPDX-License-Identifier: Apache-2.0\n"
        "*******************************************************************************\n"
        "-->\n"
    )
    test_file = tmp_path / "file.md"
    test_file.write_text(old_style_header + "\n# Heading\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"md": md_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        force=True,
    )

    fixed = test_file.read_text(encoding="utf-8")
    expected_header = md_template.format(year=datetime.now().year, author="Author")
    assert results["fixed"] == 1
    assert fixed == expected_header + "\n# Heading\n"
    assert fixed.count("<!--") == 1
    assert fixed.count("-->") == 1


def test_process_files_fix_force_rewrites_old_style_rst_header_cleanly(tmp_path):
    """Regression test for a real bug: an `.rst` file with an OLDER header
    style -- just a bare ``..`` marker followed by plain indented text with
    no ``#``-per-line/border formatting at all -- must, after a forced
    `--fix`, end up with exactly ONE ``..`` marker (previously, the bare
    ``..`` -- not recognized as part of the header block since it doesn't
    look like a border-fill line -- was reported as "leading junk" and
    blindly re-appended after the new header, which already starts with its
    own ``..``, producing a duplicate ``..`` line between the header and the
    following heading)."""
    cr_checker = load_cr_checker_module()
    rst_template = load_template("rst")
    config = write_config(tmp_path, "Author")
    old_style_header = (
        "..\n"
        "   Copyright (c) 2026 Contributors to the Eclipse Foundation\n"
        "\n"
        "   See the NOTICE file(s) distributed with this work for additional\n"
        "   information regarding copyright ownership.\n"
        "\n"
        "   This program and the accompanying materials are made available under the\n"
        "   terms of the Apache License Version 2.0 which is available at\n"
        "   https://www.apache.org/licenses/LICENSE-2.0\n"
        "\n"
        "   SPDX-License-Identifier: Apache-2.0\n"
    )
    test_file = tmp_path / "file.rst"
    test_file.write_text(old_style_header + "\nHeading\n=======\n", encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"rst": rst_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        force=True,
    )

    fixed = test_file.read_text(encoding="utf-8")
    expected_header = rst_template.format(year=datetime.now().year, author="Author")
    assert results["fixed"] == 1
    assert fixed == expected_header + "\nHeading\n=======\n"
    assert fixed.count("..\n") == 1


# --- --offset (manual preamble override) ------------------------------------


def test_locate_header_manual_offset_protects_unrecognized_preamble():
    """`--offset` force-treats the given number of characters (plus any
    trailing blank lines) as a recognized preamble, even for content
    `PREFIX_MATCHERS` doesn't know about (e.g. a custom marker) -- so a
    compliant header right after it is COMPLIANT, not MISPLACED."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=datetime.now().year, author="Author")
    marker = "@@CUSTOM_MARKER@@\n"
    content = marker + header + "print('hi')\n"

    # Without --offset, the marker is unrecognized junk -> MISPLACED.
    layout_auto = cr_checker.locate_header(content)
    status_auto, _ = cr_checker.classify(layout_auto, header_template, None)
    assert status_auto is cr_checker.Status.MISPLACED

    # With --offset covering exactly the marker, it's a protected preamble.
    layout_manual = cr_checker.locate_header(content, manual_prefix_offset=len(marker))
    status_manual, _ = cr_checker.classify(layout_manual, header_template, None)
    assert status_manual is cr_checker.Status.COMPLIANT
    assert layout_manual.leading_junk == ""
    assert layout_manual.prefix_kind == "manual"


def test_process_files_fix_with_offset_inserts_header_after_marker(tmp_path):
    """End-to-end: `--offset` also drives `--fix`, inserting the new header
    immediately after the forced preamble length rather than at char 0."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    marker = "@@CUSTOM_MARKER@@\n"
    body = "print('hi')\n"
    test_file = tmp_path / "file.py"
    test_file.write_text(marker + body, encoding="utf-8")

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=len(marker),
        remove_offset=0,
    )

    expected_header = header_template.format(year=datetime.now().year, author="Author")
    fixed = test_file.read_text(encoding="utf-8")
    assert results["fixed"] == 1
    assert fixed == marker + expected_header + "\n" + body


# --- CRLF line endings -------------------------------------------------------


def test_process_files_accepts_compliant_header_with_crlf(tmp_path):
    """A file using CRLF line endings throughout (header included) must be
    recognized as compliant -- `_normalize_line_endings`/`_restore_line_endings`
    exist precisely so CRLF files aren't misdiagnosed just because templates
    and regexes are LF-based internally."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=datetime.now().year, author="Author")
    content_lf = header + "print('hi')\n"
    test_file = tmp_path / "file.py"
    test_file.write_bytes(content_lf.replace("\n", "\r\n").encode("utf-8"))

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        False,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["missing"] == 0


def test_process_files_fix_preserves_crlf_line_endings(tmp_path):
    """`--fix` on a CRLF file must insert a CRLF-terminated header and leave
    the rest of the file CRLF throughout -- never a mix of `\\n` and `\\r\\n`."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    body_lf = "print('hi')\n"
    test_file = tmp_path / "file.py"
    test_file.write_bytes(body_lf.replace("\n", "\r\n").encode("utf-8"))

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    fixed_bytes = test_file.read_bytes()
    fixed_text = fixed_bytes.decode("utf-8")
    assert results["fixed"] == 1
    assert "\r\n" in fixed_text
    # No bare LF anywhere -- every line ending must be CRLF (a legitimate
    # blank line, i.e. two consecutive "\n", correctly becomes "\r\n\r\n",
    # which -- as an inherent, harmless side effect -- does contain the
    # substring "\n\r"; the real invariant is that nothing but CRLF pairs
    # remains once every "\r\n" occurrence is stripped out).
    assert fixed_text.replace("\r\n", "").count("\n") == 0
    assert fixed_text.endswith("print('hi')\r\n")


# --- BOM handling -------------------------------------------------------------


def test_process_files_accepts_compliant_header_with_bom(tmp_path):
    """A leading UTF-8 BOM must not make an otherwise-compliant header look
    missing (see `_strip_bom`'s docstring)."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    header = header_template.format(year=datetime.now().year, author="Author")
    test_file = tmp_path / "file.py"
    test_file.write_bytes(("\ufeff" + header + "print('hi')\n").encode("utf-8"))

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        False,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    assert results["missing"] == 0


def test_process_files_fix_preserves_leading_bom(tmp_path):
    """`--fix` on a file with a leading BOM must restore the BOM verbatim at
    the very start of the rewritten file (see `_process_file_fix`'s
    `had_bom` handling), not drop or duplicate it."""
    cr_checker = load_cr_checker_module()
    header_template = load_template("py")
    config = write_config(tmp_path, "Author")
    body = "print('hi')\n"
    test_file = tmp_path / "file.py"
    test_file.write_bytes(("\ufeff" + body).encode("utf-8"))

    results = cr_checker.process_files(
        [test_file],
        {"py": header_template},
        True,
        config=config,
        use_mmap=False,
        encoding="utf-8",
        offset=0,
        remove_offset=0,
    )

    fixed_bytes = test_file.read_bytes()
    expected_header = header_template.format(year=datetime.now().year, author="Author")
    assert results["fixed"] == 1
    assert fixed_bytes.startswith("\ufeff".encode("utf-8"))
    fixed_text = fixed_bytes.decode("utf-8")
    assert fixed_text == "\ufeff" + expected_header + "\n" + body
    assert fixed_text.count("\ufeff") == 1


# --- _atomic_write failure cleanup -------------------------------------------


def test_atomic_write_removes_temp_file_on_failure(tmp_path, monkeypatch):
    """If the final rename/move fails partway (disk full, permissions,
    ...), `_atomic_write` must not leave a stray temp file behind in the
    target directory, and the original file must remain untouched."""
    cr_checker = load_cr_checker_module()
    target = tmp_path / "file.txt"
    target.write_text("original\n", encoding="utf-8")

    def _boom(*_args, **_kwargs):
        raise OSError("simulated disk failure")

    monkeypatch.setattr(cr_checker.shutil, "move", _boom)

    with pytest.raises(OSError):
        cr_checker._atomic_write(target, "new content\n", "utf-8")

    assert target.read_text(encoding="utf-8") == "original\n"
    leftover = [p for p in tmp_path.iterdir() if p != target]
    assert leftover == []


def test_atomic_write_succeeds_and_leaves_no_temp_file(tmp_path):
    """Sanity check for the happy path: the temp file used during the write
    must not remain once `_atomic_write` returns successfully."""
    cr_checker = load_cr_checker_module()
    target = tmp_path / "file.txt"
    target.write_text("original\n", encoding="utf-8")

    cr_checker._atomic_write(target, "new content\n", "utf-8")

    assert target.read_text(encoding="utf-8") == "new content\n"
    leftover = [p for p in tmp_path.iterdir() if p != target]
    assert leftover == []
