#!/usr/bin/env python3

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
"""The tool for checking if artifacts have a proper license header."""

import argparse
import contextlib
import enum
import functools
import json
import logging
import mmap
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from rapidfuzz import fuzz

BYTES_TO_READ = 4 * 1024
DEFAULT_AUTHOR = "Contributors to the Eclipse Foundation"

BORDER_FILL_PATTERN = re.compile(r"([/*#'\-=+])\1{4,}")
FILL_CHARS_REGEX = r"[/*#'\-=+]+"

# Minimum rapidfuzz similarity (0-100) an existing, wrong-format header must
# have against the rendered template for `--fix` to treat it as a formatting
# drift (border style, indentation, a stray typo, ...) that is safe to strip
# and reformat automatically. Headers scoring below this are left untouched
# and merely reported, since they may be a genuinely different license text
# that must never be silently overwritten.
HEADER_SIMILARITY_THRESHOLD = 80.0

# Max chars allowed between "Copyright" and "SPDX-License-Identifier" for them
# to be considered the same header block. The longest real gap across every
# section of the shipped templates.ini is ~365 chars; 512 leaves comfortable
# room for a longer author name or a "YYYY-YYYY" range without ever letting
# the match wander across unrelated content (e.g. a whole file body) to reach
# a coincidental, later mention of both words.
COPYRIGHT_BLOCK_MAX_GAP = 512
# A trailing ``: <license-expr>`` is required after "SPDX-License-Identifier"
# so plain-English mentions of the phrase (e.g. in a docstring or comment
# explaining what this tool does) aren't mistaken for an actual SPDX tag.
COPYRIGHT_BLOCK_PATTERN = re.compile(
    rf"Copyright.{{0,{COPYRIGHT_BLOCK_MAX_GAP}}}?SPDX-License-Identifier\s*:[^\n]*\n?",
    re.IGNORECASE | re.DOTALL,
)
SPDX_IDENTIFIER_PATTERN = re.compile(r"SPDX-License-Identifier\s*:\s*([^\n]*)", re.IGNORECASE)
# Chars ignored when comparing two SPDX identifiers, so harmless formatting
# variants ("Apache-2.0" vs "Apache 2.0" vs "apache2.0") aren't mistaken for a
# genuine license difference ("MIT" vs "Apache-2.0").
SPDX_IGNORED_CHARS_PATTERN = re.compile(r"[\s.\-]+")

LOGGER = logging.getLogger()

COLORS = {
    "BLUE": "\033[34m",
    "GREEN": "\033[32m",
    "YELLOW": "\033[33m",
    "RED": "\033[31m",
    "DARK_RED": "\033[35;1m",
    "ENDC": "\033[0m",
}

LOGGER_COLORS = {
    "DEBUG": COLORS["BLUE"],
    "INFO": COLORS["GREEN"],
    "WARNING": COLORS["YELLOW"],
    "ERROR": COLORS["RED"],
    "CRITICAL": COLORS["DARK_RED"],
}


class ColoredFormatter(logging.Formatter):
    """
    A custom logging formatter to add color to log level names based on the logging level.

    The `ColoredFormatter` class extends `logging.Formatter` and overrides the `format`
    method to add color codes to the log level name (e.g., `INFO`, `WARNING`, `ERROR`)
    based on a predefined color mapping in `LOGGER_COLORS`. This color coding helps in
    visually distinguishing log messages by severity.

    Attributes:
        LOGGER_COLORS (dict): A dictionary mapping log level names (e.g., "INFO", "ERROR")
                              to their respective color codes.
        COLORS (dict): A dictionary of terminal color codes, including an "ENDC" key to reset
                       colors after the level name.

    Methods:
        format(record): Adds color to the `levelname` attribute of the log record and then
                        formats the record as per the superclass `Formatter`.
    """

    def format(self, record):
        log_color = LOGGER_COLORS.get(record.levelname, "")
        record.levelname = f"{log_color}{record.levelname}:{COLORS['ENDC']}"
        return super().format(record)


class ParamFileAction(argparse.Action):  # pylint: disable=too-few-public-methods
    """
    A custom argparse action to support exclusive parameter files for command-line arguments.

    The `ParamFileAction` class allows users to specify a parameter file (prefixed with '@')
    containing file paths or other inputs, which will override any additional inputs provided
    in the command line. If a parameter file is found, its contents are used exclusively,
    and all other inputs are ignored. If no parameter file is provided, standard inputs are used.

    Attributes:
        parser (argparse.ArgumentParser): The argument parser instance.
        namespace (argparse.Namespace): The namespace where arguments are stored.
        values (list): The list of argument values passed from the command line.
        option_string (str, optional): The option string that triggered this action, if any.

    Methods:
        __call__(parser, namespace, values, option_string=None): Processes the arguments.
            - If any value starts with '@', it reads the parameter file and sets `file_paths`
              in `namespace`.
            - If no parameter file is detected, it directly assigns `values` to `namespace`.
    """

    def __call__(self, parser, namespace, values, option_string=None):
        paramfile = next((v[1:] for v in values if v.startswith("@")), None)
        if paramfile:
            with open(paramfile, "r", encoding="utf-8") as handle:
                file_paths = [line.strip() for line in handle if line.strip()]
            setattr(namespace, self.dest, file_paths)
        else:
            setattr(namespace, self.dest, values)


def get_author_from_config(config_path: Path = None) -> str:
    """
    Reads the author from a JSON configuration file.

    Args:
        config_path (Path): Path to the configuration JSON file.

    Returns:
        str: Author from the configuration file.
    """
    if not config_path:
        return DEFAULT_AUTHOR
    with config_path.open("r") as file:
        config = json.load(file)
    return config.get("author", DEFAULT_AUTHOR)


def convert_bre_to_regex(template: str) -> str:
    """
    Convert BRE-style template (literal by default) to standard regex.
    In the template: * is literal, \\* is a metacharacter.
    """
    # First, escape all regex metacharacters to make them literal
    escaped = re.escape(template)
    # Now, find escaped backslashes followed by escaped metacharacters
    # and convert them back to actual regex metacharacters
    metacharacters = r"\\.*+-?[]{}()^$|"
    for char in metacharacters:
        escaped = escaped.replace(re.escape("\\" + char), char)
    return escaped


def line_to_flexible_regex(line: str) -> str:
    """
    Convert a border line to a regex that accepts any fill characters.

    Runs of 5+ identical fill characters (e.g. ``****``) are replaced with
    ``[/*#'\\-=+]+`` so that alternative styles (e.g. ``////``) are also
    accepted.
    """
    stripped = line.rstrip("\n")
    has_newline = line.endswith("\n")
    result = []
    last_end = 0
    for m in BORDER_FILL_PATTERN.finditer(stripped):
        result.append(re.escape(stripped[last_end : m.start()]))
        result.append(FILL_CHARS_REGEX)
        last_end = m.end()
    result.append(re.escape(stripped[last_end:]))
    if has_newline:
        result.append("\n")
    return "".join(result)


def load_templates(path):
    """
    Loads the copyright templates from a configuration file.

    Args:
        path (str): Path to the template file.

    Returns:
        dict: A dictionary where each key is a file extension (e.g., ".cpp")
              and the value is the template string from the config.
    """

    def add_template_for_extensions(templates: dict, extensions: list, template: str):
        # Remove trailing lines from template and ensure line end
        template = template.rstrip() + "\n"
        for extension in extensions:
            templates[extension] = template

    templates = {}
    current_extensions = []

    with open(path, "r", encoding="utf-8") as file:
        lines = file.readlines()
        template_for_extensions = ""

        for line in lines:
            stripped_line = line.strip()

            if stripped_line.startswith("[") and stripped_line.endswith("]"):
                add_template_for_extensions(templates, current_extensions, template_for_extensions)

                template_for_extensions = ""

                extensions = stripped_line[1:-1].split(",")
                current_extensions = [ext.strip() for ext in extensions]
                LOGGER.debug(current_extensions)
            else:
                template_for_extensions += line

        add_template_for_extensions(templates, current_extensions, template_for_extensions)

    LOGGER.debug(templates)
    return templates


def load_exclusion(path, base_dir=None):
    """
    Loads the list of files being excluded from the copyright check.

    Args:
        path (str): Path to the exclusion file.
        base_dir (str, optional): Directory the (repo-relative) exclusion entries are
                                  resolved against. When set, the returned paths are
                                  absolute so they can be matched against the resolved
                                  input file paths.

    Returns:
        tuple(list, bool): a list of files that are excluded from the copyright check and a boolean indicating whether
                           all paths listed in the exclusion file exist and are files.
    """

    resolved = []
    valid = True
    with open(path, "r", encoding="utf-8") as file:
        entries = file.read().splitlines()

    for item in entries:
        if not item:
            continue
        candidate = item
        if base_dir and not os.path.isabs(candidate):
            candidate = os.path.join(base_dir, candidate)
        candidate_path = Path(candidate)
        if not candidate_path.exists():
            LOGGER.error("Excluded file %s does not exist.", item)
            valid = False
            continue
        if not candidate_path.is_file():
            LOGGER.error("Excluded file %s is not a file.", item)
            valid = False
            continue
        # Resolved so matching against a file's own resolved path (see
        # `process_files`) isn't tripped up by relative-vs-absolute or
        # symlink differences between the two representations of the same
        # file.
        resolved.append(str(candidate_path.resolve()))

    LOGGER.debug(resolved)
    return resolved, valid


def configure_logging(log_file_path=None, verbose=False):
    """
    Configures logging to write messages to the specified log file.

    Args:
        log_file_path (str, optional): Path to the log file.
        verbose (bool, optional): If True, sets log level to DEBUG. Otherwise, sets it to INFO.
    """
    log_level = logging.DEBUG if verbose else logging.INFO
    LOGGER.setLevel(log_level)
    LOGGER.handlers.clear()

    if log_file_path is not None:
        handler = logging.FileHandler(log_file_path)
        formatter = logging.Formatter("%(levelname)s: %(message)s")
    else:
        handler = logging.StreamHandler()
        formatter = ColoredFormatter("%(levelname)s %(message)s")

    handler.setLevel(log_level)
    handler.setFormatter(formatter)
    LOGGER.addHandler(handler)


BOM = "\ufeff"


def _strip_bom(text):
    """Strips a leading UTF-8 BOM, if present.

    Without this, a BOM would sit at char offset 0 and every offset-0-anchored
    header match would spuriously fail, making a compliant file look like it's
    missing its header.

    Returns:
        tuple(str, bool): the text without a leading BOM, and whether one was
        present (so a fix can restore it verbatim on write).
    """
    if text.startswith(BOM):
        return text[len(BOM) :], True
    return text, False


def _detect_line_ending(text):
    """Returns the dominant line ending ("\\r\\n" or "\\n") used by `text`."""
    return "\r\n" if "\r\n" in text else "\n"


def _normalize_line_endings(text):
    """Converts CRLF to LF so all scanning/matching only ever has to deal with
    a single line-ending convention (templates and regexes are LF-based)."""
    return text.replace("\r\n", "\n")


def _restore_line_endings(text, line_ending):
    """Converts LF back to `line_ending` (a no-op for LF files)."""
    return text if line_ending == "\n" else text.replace("\n", line_ending)


def _skip_blank_lines(text, pos):
    """Advances `pos` in `text` (LF-normalized) past any blank lines starting
    right there, returning the new position."""
    while pos < len(text):
        line_end = text.find("\n", pos)
        line = text[pos : line_end + 1] if line_end != -1 else text[pos:]
        if line.strip():
            break
        pos = line_end + 1 if line_end != -1 else len(text)
    return pos


def _match_shebang(text):
    """Prefix matcher: returns the char length of a leading shebang line
    (including its trailing newline), or 0 if `text` doesn't start with one.

    `#![...]` is Rust's inner-attribute syntax (e.g. `#![cfg_attr(...)]`), not
    a shebang, even though it also starts with `#!`; a real shebang is always
    followed directly by an interpreter path (`#!/usr/bin/env ...`) or a
    space before one (`#! /usr/bin/env ...`), never `[`.
    """
    if not text.startswith("#!") or text.startswith("#!["):
        return 0
    line_end = text.find("\n")
    return len(text) if line_end == -1 else line_end + 1


# Ordered registry of recognized "must stay above the header" preamble kinds.
# Each entry is (name, matcher) where matcher(text) returns the number of
# characters it consumes at the current start-of-file position, or 0 if it
# doesn't apply. This is the single place to extend with new preamble kinds
# later (e.g. a PEP 263 encoding cookie, an XML declaration, ...) without
# touching the scanning/classification/fix logic itself. Currently only the
# shebang is recognized.
PREFIX_MATCHERS = [
    ("shebang", _match_shebang),
]


def _match_prefix(text):
    """Runs the prefix matcher registry once at the top of `text`.

    Returns:
        tuple(int, str | None): the char offset where the recognized preamble
        (plus any immediately trailing blank lines) ends, and its kind (e.g.
        "shebang"), or (0, None) if nothing matched.
    """
    for name, matcher in PREFIX_MATCHERS:
        consumed = matcher(text)
        if consumed:
            return _skip_blank_lines(text, consumed), name
    return 0, None


def load_text_from_file(path, length, encoding):
    """
    Reads the first `length` characters of a file, without translating line
    endings, so callers can detect/normalize them themselves.

    Args:
        path (Path): A `pathlib.Path` object pointing to the file.
        length (int): Number of characters to read.
        encoding (str): Encoding type to use when reading the file.

    Returns:
        str: The portion of the file read.
    """
    LOGGER.debug("Reading first %d characters from file: %s [%s]", length, path, encoding)
    with open(path, "r", encoding=encoding, newline="") as handle:
        return handle.read(length)


def load_text_from_file_with_mmap(path, length, encoding):
    """
    Maps the file and reads only the first `length` bytes.

    Args:
        path (Path): A `pathlib.Path` object pointing to the file.
        length (int): Length of the header text to check.
        encoding (str): String for setting decoding type.

    Returns:
        str: The portion of the file read.
    """
    file_size = os.path.getsize(path)
    length = min(length, file_size)

    if not length:
        LOGGER.warning("File %s is empty [length: %d]. Return empty string.", path, length)
        return ""

    LOGGER.debug("Memory mapping first %d bytes from file: %s", length, path)
    with open(path, "r", encoding=encoding, newline="") as handle:
        with mmap.mmap(handle.fileno(), length=length, access=mmap.ACCESS_READ) as fmap:
            return fmap[:length].decode(encoding, errors="replace")


def load_header_text(path, length, encoding, use_mmap=False):
    """
    Reads the file header once, dispatching between a plain read and a
    memory-mapped read depending on `use_mmap`.

    Args:
        path (Path): A `pathlib.Path` object pointing to the file to read.
        length (int): Number of characters/bytes to read for the header.
        encoding (str): Encoding type to use when reading the file.
        use_mmap (bool): If True, uses memory-mapped file reading for efficient
                         large file handling.

    Returns:
        str: The portion of the file read, which should contain the header if present.
    """
    reader = load_text_from_file_with_mmap if use_mmap else load_text_from_file
    return reader(path, length, encoding)


@dataclass(frozen=True)
class Block:
    """A copyright-shaped block (matched by ``COPYRIGHT_BLOCK_PATTERN``)
    found while scanning a file, including any directly adjacent border/fill
    lines and trailing blank lines -- i.e. the whole header block a tool like
    this one would have produced, not just the legal text itself."""

    start: int
    end: int
    text: str


@dataclass(frozen=True)
class HeaderLayout:
    """Where everything is in a file's header area, from a single scan.

    This is the single "where is everything" model shared by both the
    read-only check path (`classify`) and the mutating fix path
    (`normalize_header`), replacing what used to be two independently
    derived views: shebang-offset detection for the missing-header case, and
    block-span search for the wrong-format case.

    Attributes:
        prefix_end (int): Char offset where the recognized preamble (see
                          `PREFIX_MATCHERS`), plus any trailing blank lines,
                          ends. 0 if nothing was recognized.
        prefix_kind (str | None): Which preamble kind matched (e.g.
                                  "shebang", or "manual" for `--offset`), or
                                  None.
        blocks (list[Block]): Every copyright-shaped block found at or after
                              `prefix_end`, in file order. Empty means no
                              header was found at all (MISSING); more than
                              one means DUPLICATE.
        leading_junk (str): Real, non-blank content sitting between
                            `prefix_end` and `blocks[0].start` -- i.e.
                            something that isn't the recognized preamble and
                            isn't the header itself. Empty when there's
                            nothing there, or when `blocks` is empty (a fully
                            missing header has nothing to be "misplaced"
                            relative to).
    """

    prefix_end: int
    prefix_kind: str | None
    blocks: list
    leading_junk: str


def _find_blocks(text, start):
    """Finds every copyright-shaped block in `text` from `start` onward.

    Mirrors the old `find_header_block_span`, looped to collect every match
    (needed to tell a single misplaced-but-correct header apart from
    genuine duplicates) instead of stopping at the first.
    """
    lines = text.splitlines(keepends=True)
    line_offsets = [0]
    for line in lines:
        line_offsets.append(line_offsets[-1] + len(line))

    blocks = []
    search_from = start
    while True:
        match = COPYRIGHT_BLOCK_PATTERN.search(text, search_from)
        if not match:
            break

        start_idx = next(i for i, off in enumerate(line_offsets) if off > match.start()) - 1
        end_idx = next((i for i, off in enumerate(line_offsets) if off >= match.end()), len(lines))

        while start_idx > 0 and BORDER_FILL_PATTERN.search(lines[start_idx - 1].strip()):
            start_idx -= 1
        while end_idx < len(lines) and BORDER_FILL_PATTERN.search(lines[end_idx].strip()):
            end_idx += 1
        while end_idx < len(lines) and not lines[end_idx].strip():
            end_idx += 1

        block_start, block_end = line_offsets[start_idx], line_offsets[end_idx]
        blocks.append(Block(block_start, block_end, text[block_start:block_end]))
        search_from = max(block_end, match.end())

    return blocks


def locate_header(text, manual_prefix_offset=0):
    """Scans `text` once and returns where everything is (see `HeaderLayout`).

    Shared verbatim by both the check path (`classify`) and the fix path
    (`normalize_header`) -- the single source of truth for "what's a
    recognized preamble, what's the header, what's neither" that used to be
    computed two different, independently-evolving ways.

    Args:
        text (str): The decoded, BOM-stripped, LF-normalized content to scan
                    (a bounded header window in check mode, or the whole
                    file in fix mode).
        manual_prefix_offset (int): If set (via `--offset`), overrides
                                    auto-detection and forces this many
                                    characters (plus trailing blank lines) to
                                    be treated as the recognized preamble --
                                    an escape hatch for preamble kinds the
                                    registry doesn't (yet) recognize.

    Returns:
        HeaderLayout: see above.
    """
    if manual_prefix_offset:
        prefix_end, prefix_kind = _skip_blank_lines(text, manual_prefix_offset), "manual"
    else:
        prefix_end, prefix_kind = _match_prefix(text)

    blocks = _find_blocks(text, prefix_end)

    leading_junk = ""
    if blocks:
        candidate = text[prefix_end : blocks[0].start]
        if candidate.strip():
            leading_junk = candidate

    return HeaderLayout(prefix_end, prefix_kind, blocks, leading_junk)


class Status(enum.Enum):
    """Classification of a file's header state (check-mode diagnostics)."""

    MISSING = "missing"
    COMPLIANT = "compliant"
    MISPLACED = "misplaced"
    WRONG_FORMAT = "wrong_format"
    MISPLACED_AND_WRONG_FORMAT = "misplaced_and_wrong_format"
    DUPLICATE = "duplicate"
    LICENSE_MISMATCH = "license_mismatch"


# Statuses `--fix` is allowed to act on automatically. WRONG_FORMAT and
# MISPLACED_AND_WRONG_FORMAT are additionally gated on the similarity
# threshold at the call site (see `_process_file_fix`) -- a low score means
# the text is probably an unrelated license, which must never be silently
# rewritten. LICENSE_MISMATCH, like DUPLICATE, is never in this set --
# unlike the similarity gate, `--force` cannot bypass it either.
FIXABLE_STATUSES = {
    Status.MISSING,
    Status.MISPLACED,
    Status.WRONG_FORMAT,
    Status.MISPLACED_AND_WRONG_FORMAT,
}


@functools.lru_cache(maxsize=None)
def compile_template_regex(template):
    """
    Builds and compiles the header-matching regex for a template.

    Results are cached per template string so the (relatively expensive) regex
    construction happens once per extension rather than once per file.

    Args:
        template (str): The copyright template text.

    Returns:
        re.Pattern: The compiled regex matching a conforming header.
    """
    regex_parts = []
    for line in template.splitlines(keepends=True):
        stripped_line = line.rstrip("\n")
        if BORDER_FILL_PATTERN.search(stripped_line):
            regex_parts.append(line_to_flexible_regex(line))
        else:
            formatted = line.format(year=r"\\d\{4\}\(-\\d\{4\}\)\?", author=r"\.\*")
            regex_parts.append(convert_bre_to_regex(formatted))
    return re.compile("".join(regex_parts) + "\n?")


def _blocks_in_duplicate_window(layout, template):
    """Blocks close enough to `layout.prefix_end` to count as (part of) the
    file's real header, for both DUPLICATE detection and rewriting.

    Shared by `classify` and `normalize_header` so the two can never
    disagree about which blocks are "the header" -- previously `classify`
    only ever scored/duplicate-checked blocks within this window, while
    `normalize_header` deleted through the *last* block found anywhere in
    the file, so a coincidental copyright-shaped mention far past a
    fixable header (e.g. deep in a docstring or example) could make
    `--fix` silently delete everything in between.

    Args:
        layout (HeaderLayout): The result of `locate_header`.
        template (str): The copyright template, used to size the window
                        (``2 * len(template)`` chars from `prefix_end`).

    Returns:
        list[Block]: The leading subsequence of `layout.blocks` within the
        window, in file order.
    """
    window = 2 * len(template)
    return [b for b in layout.blocks if b.start - layout.prefix_end < window]


def _is_old_wrapper_prefix(leading_junk, template):
    """True if `leading_junk` is just a leftover fragment of an OLD-style
    header's own opening comment-wrapper marker (e.g. a bare ``<!--`` or
    ``..`` line from a previous version of the md/rst templates) that
    `_find_blocks`'s border-adjacency scan -- deliberately template-agnostic,
    so it only recognizes repeated-fill-char borders, not arbitrary comment
    delimiters -- didn't absorb into the block itself. Distinguished from
    genuine, unrelated content that happens to sit before the header (a real
    MISPLACED case) by checking whether it's a prefix of the *current*
    template's own first line: every shipped template keeps its wrapper
    marker on the first line (``<!-- ...`` for md, ``..`` for rst, ``# ...``/
    ``// ...``/... for the rest), so a match here means the file used an
    older/differently-formatted version of the very same wrapper, not
    something else entirely. Also requires the fragment to contain no
    alphanumeric characters, so real (if terse) preceding content isn't
    swallowed by coincidence.
    """
    stripped = leading_junk.strip()
    if not stripped or any(ch.isalnum() for ch in stripped):
        return False
    first_template_line = template.splitlines()[0] if template else ""
    return first_template_line.startswith(stripped)


def _strip_old_wrapper_suffix(remainder, template):
    """Drops a leading line from `remainder` if it's just a leftover
    fragment of an OLD-style header's own closing comment-wrapper marker
    (e.g. a bare ``-->`` left over from an older md header written before
    the wrapper was folded onto the border lines) that `_find_blocks`'s
    border-adjacency scan didn't absorb into the block. Symmetric
    counterpart to `_is_old_wrapper_prefix` for the closing side -- checks
    whether the candidate line is a suffix of the *current* template's own
    last line. A no-op when `remainder` doesn't start with such a fragment.
    """
    newline_idx = remainder.find("\n")
    first_line = remainder if newline_idx == -1 else remainder[: newline_idx + 1]
    stripped = first_line.strip()
    if not stripped or any(ch.isalnum() for ch in stripped):
        return remainder
    last_template_line = template.splitlines()[-1] if template else ""
    if not last_template_line.endswith(stripped):
        return remainder
    return remainder[len(first_line) :].lstrip("\n")


def _extract_spdx_id(text):
    """Returns the raw ``SPDX-License-Identifier`` value in `text` (e.g.
    ``"Apache-2.0"``), or None if it doesn't contain one."""
    match = SPDX_IDENTIFIER_PATTERN.search(text)
    return match.group(1).strip() if match else None


def _normalize_spdx_id(spdx_id):
    """Folds an SPDX identifier down for lenient comparison (see
    `SPDX_IGNORED_CHARS_PATTERN`): case-insensitive, ignoring spaces, dots
    and hyphens."""
    return SPDX_IGNORED_CHARS_PATTERN.sub("", spdx_id).casefold()


def _spdx_mismatch(existing_text, template):
    """True if `existing_text` and `template` each carry an SPDX identifier
    and they genuinely differ (e.g. ``MIT`` vs ``Apache-2.0``) -- as opposed
    to just a formatting variant of the same one (see `_normalize_spdx_id`).
    False if either side has no SPDX identifier at all, since that's not
    this check's concern (missing/wrong-format handling already covers it).
    """
    existing = _extract_spdx_id(existing_text)
    expected = _extract_spdx_id(template)
    if not existing or not expected:
        return False
    return _normalize_spdx_id(existing) != _normalize_spdx_id(expected)


def classify(layout, template, config):
    """Classifies a file's header state from its `HeaderLayout`.

    Args:
        layout (HeaderLayout): The result of `locate_header`.
        template (str): The copyright template the header should match.
        config (Path | None): Path to the config JSON file, used to render
                              the template's placeholders for comparison.

    Returns:
        tuple(Status, float | None): the status, and (when there's exactly
        one block to score) the rapidfuzz similarity (0-100) between that
        block and the rendered template -- used both for the
        WRONG_FORMAT/MISPLACED_AND_WRONG_FORMAT diagnostic and to gate
        whether `--fix` may safely rewrite it. None for MISSING/DUPLICATE,
        which have no single block to score.

    Note:
        Only blocks starting within ``2 * len(template)`` characters of
        ``layout.prefix_end`` count toward DUPLICATE detection -- the same
        window the original implementation used -- so that a copyright-shaped
        mention deep inside a docstring/comment/string literal further into
        the file (e.g. an example header, or literally this module's own
        ``COPYRIGHT_BLOCK_PATTERN`` regex source) isn't mistaken for a
        duplicate of a real header near the top.

        A wrong-format header whose own SPDX identifier genuinely differs
        from the template's (see `_spdx_mismatch`) is classified
        LICENSE_MISMATCH instead of WRONG_FORMAT/MISPLACED_AND_WRONG_FORMAT,
        regardless of how similar the rest of the boilerplate text looks --
        a whole-block similarity score alone can't be trusted to catch this,
        since a short header is mostly shared boilerplate around the one
        line that actually matters.
    """
    if not layout.blocks:
        return Status.MISSING, None

    blocks_in_window = _blocks_in_duplicate_window(layout, template)
    if len(blocks_in_window) > 1:
        return Status.DUPLICATE, None

    block = layout.blocks[0]
    rendered = template.format(year=datetime.now().year, author=get_author_from_config(config))
    template_regex = compile_template_regex(template)

    # Some templates have a literal, non-border prefix line before the
    # border itself (e.g. the RST comment marker ".." before the "# ****"
    # border), which `_find_blocks`'s border-adjacency scan -- deliberately
    # template-agnostic -- doesn't recognize and so reports as
    # `leading_junk`. Before treating it as genuine misplaced content, check
    # whether the junk plus the block together are actually just the
    # template's own (longer) literal prefix.
    if layout.leading_junk:
        extended = layout.leading_junk + block.text
        if template_regex.match(extended):
            return Status.COMPLIANT, fuzz.ratio(extended, rendered)

    similarity = fuzz.ratio(block.text, rendered)
    correct_format = bool(template_regex.match(block.text))
    misplaced = bool(layout.leading_junk) and not _is_old_wrapper_prefix(layout.leading_junk, template)

    if not correct_format and _spdx_mismatch(block.text, template):
        return Status.LICENSE_MISMATCH, similarity
    if correct_format and not misplaced:
        return Status.COMPLIANT, similarity
    if correct_format and misplaced:
        return Status.MISPLACED, similarity
    if misplaced:
        return Status.MISPLACED_AND_WRONG_FORMAT, similarity
    return Status.WRONG_FORMAT, similarity


def duplicate_similarity(blocks):
    """
    Best-effort rapidfuzz similarity (0-100) between the two most similar
    detected blocks, for diagnostics only.

    `classify`'s DUPLICATE trigger is simply "more than one block found" --
    by design, cross-tool duplicates such as a leftover REUSE-style header
    sitting next to a cr_checker one are structurally very different, so
    gating *detection* on similarity would regress that case. This helper
    only *describes* what was found, for the log message: a high score means
    the same header was likely pasted twice (safe to just delete the extra
    copy); a low score means two structurally different header blocks are
    present (e.g. a REUSE/cr_checker leftover pair), which need a manual
    merge rather than a blind deletion.

    Args:
        blocks (list[Block]): The blocks found by `locate_header`.

    Returns:
        float | None: The highest pairwise similarity among the blocks, or
        None if fewer than two were found.
    """
    if len(blocks) < 2:
        return None
    return max(fuzz.ratio(a.text, b.text) for i, a in enumerate(blocks) for b in blocks[i + 1 :])


def extension_key(path):
    """Returns the template/extension-filter lookup key for `path`.

    The key is the file extension without the leading dot; the extension-less
    ``BUILD`` file is special-cased to use its literal name instead. This is
    the single source of truth for that rule, shared by template lookup
    (`process_files`) and extension filtering (`_matches_extension`).
    """
    path = Path(path)
    return path.name if path.name == "BUILD" else path.suffix[1:]


def _matches_extension(path, exts):
    """Returns whether ``path`` should be considered for the given extension filter.

    ``exts`` is the list passed via ``--extensions``. ``None`` means "no filter".
    """
    if exts is None:
        return True
    return extension_key(path) in exts


def list_tracked_files(base_dir, pathspecs=None):
    """Lists repository files via ``git ls-files`` under ``base_dir``.

    Uses git as the source of truth for "which files exist in the repo":

    * ``--cached --others --exclude-standard`` yields tracked plus untracked
      files while honoring ``.gitignore`` (so Bazel ``bazel-*`` convenience
      symlinks and other generated artifacts are skipped automatically).
    * git does not respect Bazel package boundaries, so a single call reaches
      files in nested packages without any per-package configuration.

    Args:
        base_dir (str): Repository root to run git in.
        pathspecs (list, optional): git pathspecs (directories or globs) to
                                    restrict the listing. ``None`` lists the
                                    whole repository.

    Returns:
        list[Path]: Absolute paths to the listed files.
    """
    cmd = [
        "git",
        "-C",
        str(base_dir),
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ]
    if pathspecs:
        cmd.append("--")
        cmd.extend(pathspecs)

    LOGGER.debug("Listing files with: %s", " ".join(cmd))
    result = subprocess.run(cmd, capture_output=True, check=True)
    base = Path(base_dir)
    return [base / os.fsdecode(rel) for rel in result.stdout.split(b"\0") if rel]


def list_modified_files(base_dir):
    """Lists files added, copied, modified, or renamed relative to ``HEAD``.

    Used by ``--modified-only`` for fast, incremental runs (e.g. a
    pre-commit hook) that should only check what's actually changing,
    instead of the whole repository. Compares the working tree directly to
    ``HEAD`` (not just the index), so it covers both staged and unstaged
    changes to tracked files, plus new files once they're staged (`git
    diff` never reports a file it has no record of at all, i.e. a brand
    new file that was never `git add`ed); deleted files are excluded since
    there's nothing left to check. This matches how pre-commit itself only
    ever hands hooks the files staged for the commit.

    Args:
        base_dir (str): Repository root to run git in.

    Returns:
        list[Path]: Absolute paths to the modified files.
    """
    cmd = [
        "git",
        "-C",
        str(base_dir),
        "diff",
        "--name-only",
        "--diff-filter=ACMR",
        "-z",
        "HEAD",
    ]

    LOGGER.debug("Listing modified files with: %s", " ".join(cmd))
    result = subprocess.run(cmd, capture_output=True, check=True)
    base = Path(base_dir)
    return [base / os.fsdecode(rel) for rel in result.stdout.split(b"\0") if rel]


def collect_inputs(inputs, exts=None, base_dir=None):
    """Collects files to check from the given inputs, filtered by extension.

    Existing files are taken as-is; every other input is treated as a git
    pathspec (a directory or glob) resolved via ``list_tracked_files``. When
    ``inputs`` is empty the whole repository is listed.

    Args:
        inputs (list): Files and/or git pathspecs (relative to ``base_dir``).
        exts (list, optional): Extensions to keep (see ``_matches_extension``).
        base_dir (str, optional): Repository root. Defaults to the current
                                  working directory.

    Returns:
        list[Path]: The files to check.
    """
    base = Path(base_dir) if base_dir else Path.cwd()
    LOGGER.debug("Extensions: %s", exts)

    explicit_files = []
    pathspecs = []
    for i in inputs:
        candidate = Path(i)
        absolute = candidate if candidate.is_absolute() else base / candidate
        if absolute.is_file():
            explicit_files.append(absolute)
        else:
            pathspecs.append(i)

    candidates = list(explicit_files)
    if pathspecs or not inputs:
        candidates.extend(list_tracked_files(base, pathspecs or None))

    collected = []
    seen = set()
    for path in candidates:
        if path.is_symlink() or not path.is_file():
            continue
        # An explicit file and an overlapping directory/pathspec (e.g. a file
        # passed both directly and via its parent directory) can list the
        # same path twice; de-duplicate so it isn't processed more than once.
        resolved = path.resolve()
        if resolved in seen:
            continue
        if _matches_extension(path, exts):
            seen.add(resolved)
            collected.append(path)
        else:
            LOGGER.debug("Skipped (no configuration for file extension): %s", path)
    return collected


def render_template(template, config):
    """Renders `template`'s `{year}`/`{author}` placeholders.

    Args:
        template (str): The copyright template text.
        config (Path | None): Path to the config JSON file (see
                              `get_author_from_config`).

    Returns:
        str: The rendered header text.
    """
    return template.format(year=datetime.now().year, author=get_author_from_config(config))


def _atomic_write(path, content, encoding):
    """Writes `content` to `path` atomically.

    Builds the full replacement in a temp file in the same directory as
    `path`, then renames it over the original -- so a crash/interrupt mid-write
    (disk full, process killed, ...) never leaves `path` truncated or
    partially written, unlike writing into `path` in-place. The temp file is
    always cleaned up: moved into place on success, removed on failure.

    Args:
        path (str): Path to the file to (re)write.
        content (str): The full new content of the file.
        encoding (str): Encoding to use when writing.
    """
    directory = os.path.dirname(os.path.abspath(path)) or "."
    descriptor, tmp_name = tempfile.mkstemp(dir=directory)
    try:
        with os.fdopen(descriptor, "w", encoding=encoding, newline="") as handle:
            handle.write(content)
        shutil.move(tmp_name, path)
    except BaseException:
        with contextlib.suppress(OSError):
            os.remove(tmp_name)
        raise


def normalize_header(text, layout, template, config):
    """Rebuilds `text` with a single, correctly rendered copyright header
    placed immediately after the recognized preamble (see `locate_header`).

    Every block within the duplicate-detection window (see
    `_blocks_in_duplicate_window` -- the same blocks `classify` treats as
    "the header") is dropped. Any block further into the file is left
    completely untouched, as ordinary body content: it was never counted as
    part of the header for classification either, so deleting through it
    would silently destroy unrelated content (e.g. a coincidental
    "Copyright...SPDX-License-Identifier:" mention deep in a docstring or
    example). `leading_junk` -- real content that isn't the recognized
    preamble and isn't a copyright block -- is preserved, but relocated to
    *after* the new header. This one routine resolves MISSING, MISPLACED,
    WRONG_FORMAT and MISPLACED_AND_WRONG_FORMAT alike, replacing what used
    to be a strip-then-reinsert composition of two separate functions.

    Args:
        text (str): The decoded, BOM-stripped, LF-normalized whole file
                    content (the caller restores the original line ending
                    style and BOM on write).
        layout (HeaderLayout): The result of `locate_header` for `text`.
        template (str): The copyright template to render.
        config (Path | None): Path to the config JSON file.

    Returns:
        str: The rebuilt, LF-normalized file content.
    """
    prefix = text[: layout.prefix_end]
    kept_blocks = _blocks_in_duplicate_window(layout, template)
    remainder_start = kept_blocks[-1].end if kept_blocks else layout.prefix_end
    remainder = _strip_old_wrapper_suffix(text[remainder_start:].lstrip("\n"), template)

    # `leading_junk` is dropped rather than preserved-after-header when it's
    # just a leftover fragment of an OLD-style header's own comment-wrapper
    # marker (see `_is_old_wrapper_prefix`) -- `render_template` already
    # emits the *current* template's own complete wrapper, so re-appending
    # this debris after it would create a duplicate/orphaned fragment
    # instead of preserving genuinely misplaced content.
    leading_junk = "" if _is_old_wrapper_prefix(layout.leading_junk, template) else layout.leading_junk

    parts = [prefix, render_template(template, config), "\n"]
    if leading_junk:
        parts.append(leading_junk)
        if not leading_junk.endswith("\n"):
            parts.append("\n")
    parts.append(remainder)
    return "".join(parts)


def _tally(status, results):
    """Bumps the appropriate counter in `results` for a classified `status`
    (see `process_files`'s return value docs). COMPLIANT contributes
    nothing."""
    if status is Status.MISSING:
        results["missing"] += 1
    elif status is Status.DUPLICATE:
        results["duplicate"] += 1
    elif status is Status.LICENSE_MISMATCH:
        results["license_mismatch"] += 1
    elif status is Status.MISPLACED:
        results["misplaced"] += 1
    elif status in (Status.WRONG_FORMAT, Status.MISPLACED_AND_WRONG_FORMAT):
        results["wrong_format"] += 1


def _log_status(item, status, layout, similarity, template):
    """Logs a diagnostic message for a classified `status`."""
    if status is Status.COMPLIANT:
        LOGGER.debug("File %s has copyright.", item)
    elif status is Status.MISSING:
        LOGGER.error("Missing copyright header in: %s, use --fix to introduce it", item)
    elif status is Status.LICENSE_MISMATCH:
        LOGGER.error(
            "License mismatch in: %s (found %r, expected %r) -- this looks like a genuinely "
            "different license and is never auto-fixed, not even with --force; resolve manually",
            item,
            _extract_spdx_id(layout.blocks[0].text),
            _extract_spdx_id(template),
        )
    elif status is Status.MISPLACED:
        LOGGER.error(
            "Copyright header in %s is correctly formatted but preceded by other content "
            "(similarity: %.0f%%); use --fix to move it to the top",
            item,
            similarity,
        )
    elif status is Status.MISPLACED_AND_WRONG_FORMAT:
        LOGGER.error(
            "Copyright header in %s is preceded by other content and doesn't match the "
            "expected format (similarity to template: %.0f%%)",
            item,
            similarity,
        )
    elif status is Status.WRONG_FORMAT:
        LOGGER.error(
            "Wrong copyright format in: %s (similarity to template: %.0f%%), expected format from template",
            item,
            similarity,
        )
    elif status is Status.DUPLICATE:
        similarity = duplicate_similarity(layout.blocks)
        if similarity is None:
            LOGGER.error("Duplicate copyright header in: %s", item)
        elif similarity >= HEADER_SIMILARITY_THRESHOLD:
            LOGGER.error(
                "Duplicate copyright header in: %s (repeated headers are %.0f%% similar -- "
                "likely the same header pasted twice)",
                item,
                similarity,
            )
        else:
            LOGGER.error(
                "Duplicate copyright header in: %s (repeated headers are only %.0f%% similar -- "
                "likely different header formats/tools; review both before merging)",
                item,
                similarity,
            )


def _process_file_check(item, template, encoding, offset, use_mmap, config, results):
    """Read-only path: classifies the file's header state and logs/tallies it."""
    header_window = max(BYTES_TO_READ, 2 * len(template))
    raw = load_header_text(item, header_window, encoding, use_mmap)
    text, _ = _strip_bom(raw)
    text = _normalize_line_endings(text)

    layout = locate_header(text, manual_prefix_offset=offset)
    status, similarity = classify(layout, template, config)
    _log_status(item, status, layout, similarity, template)
    _tally(status, results)


def _process_file_fix(item, template, encoding, offset, remove_offset, config, results, force=False):
    """Mutating path: classifies the file's header state, logs/tallies it, and
    -- for any `FIXABLE_STATUSES` result that passes the similarity guard (or
    `force`, which bypasses it) -- rewrites the file with `normalize_header`,
    atomically."""
    with open(item, "r", encoding=encoding, newline="") as handle:
        raw = handle.read()

    if remove_offset:
        raw = raw[remove_offset:]

    text, had_bom = _strip_bom(raw)
    line_ending = _detect_line_ending(text)
    text = _normalize_line_endings(text)

    layout = locate_header(text, manual_prefix_offset=offset)
    status, similarity = classify(layout, template, config)
    _log_status(item, status, layout, similarity, template)
    _tally(status, results)

    if status not in FIXABLE_STATUSES:
        return
    if (
        not force
        and status in (Status.WRONG_FORMAT, Status.MISPLACED_AND_WRONG_FORMAT)
        and similarity < HEADER_SIMILARITY_THRESHOLD
    ):
        # Likely an unrelated, genuinely different license text -- never
        # silently overwritten, unless the caller explicitly opted in via
        # --force.
        return

    new_text = normalize_header(text, layout, template, config)
    new_text = _restore_line_endings(new_text, line_ending)
    if had_bom:
        new_text = BOM + new_text
    _atomic_write(item, new_text, encoding)
    results["fixed"] += 1
    LOGGER.info("Fixed (%s) header in: %s", status.value, item)


def process_files(
    files,
    templates,
    fix,
    exclusion=None,
    config=None,
    use_mmap=False,
    encoding="utf-8",
    offset=0,
    remove_offset=0,
    force=False,
):  # pylint: disable=too-many-arguments
    """
    Processes a list of files to check for the presence of copyright text.

    Args:
        files (list): A list of file paths to check.
        templates (dict): A dictionary where keys are file extensions
                          (e.g., '.py', '.txt') and values are strings or patterns
                          representing the required copyright text.
        exclusion (list): A list of paths to files to be excluded from the copyright
                          check.
        config (Path): Path to the config JSON file where configuration
                       variables are stored (e.g. years for copyright headers).
        use_mmap (bool): Flag for using mmap function for reading files
                         (instead of standard option); ignored in ``--fix``
                         mode, which always reads the whole file.
        encoding (str): Encoding type to use when reading the file.
        offset (int): Number of characters to force-treat as a recognized
                      preamble (see `locate_header`), overriding
                      auto-detection. Typically only needed for preamble
                      kinds `PREFIX_MATCHERS` doesn't recognize.
        remove_offset(int): Number of characters to remove from the very
                            start of the file before processing, in
                            ``--fix`` mode only.
        force (bool): Bypass the ``HEADER_SIMILARITY_THRESHOLD`` guard for
                     WRONG_FORMAT/MISPLACED_AND_WRONG_FORMAT, rewriting the
                     header regardless of how different it looks from the
                     template. Only used in ``--fix`` mode. Never applies to
                     DUPLICATE or LICENSE_MISMATCH, which are always left for
                     manual review.

    Returns:
        dict: Counters for ``missing``, ``misplaced``, ``wrong_format``,
        ``duplicate``, ``license_mismatch`` and ``fixed``.

    Note:
        A wrong-format or misplaced-and-wrong-format header is only
        auto-fixed if its rapidfuzz similarity to the rendered template is
        at or above ``HEADER_SIMILARITY_THRESHOLD`` -- i.e. it looks like
        the same copyright statement with a formatting difference (border
        style, missing angle brackets, a typo, ...) -- unless ``force`` is
        set, in which case the similarity is ignored. Otherwise it is left
        untouched and only reported, since it may be a genuinely different
        license text that must never be silently overwritten. Duplicate
        headers are never auto-fixed, regardless of similarity or ``force``.
        Nor is a header whose own SPDX identifier genuinely differs from the
        template's (``LICENSE_MISMATCH``) -- similarity alone can't be
        trusted there, and ``force`` does not override it either.
    """
    if exclusion is None:
        exclusion = []
    results = {
        "missing": 0,
        "misplaced": 0,
        "wrong_format": 0,
        "duplicate": 0,
        "license_mismatch": 0,
        "fixed": 0,
    }

    for item in files:
        key = extension_key(item)
        if key not in templates:
            LOGGER.debug("Skipped (no configuration for selected file extension): %s", item)
            continue

        if str(Path(item).resolve()) in exclusion:
            LOGGER.debug("Skipped due to exclusion: %s", item)
            continue

        if os.path.getsize(item) == 0:
            # No need to add copyright headers to empty files
            continue

        template = templates[key]
        try:
            if fix:
                _process_file_fix(item, template, encoding, offset, remove_offset, config, results, force)
            else:
                _process_file_check(item, template, encoding, offset, use_mmap, config, results)
        except (IOError, OSError, UnicodeError) as err:
            # A single unreadable/undecodable file (permissions, binary
            # content matched by an over-broad -e filter, ...) shouldn't
            # abort the whole batch -- log it and keep going.
            LOGGER.error("Failed to process %s: %s", item, err)

    return results


def parse_arguments(argv):
    """
    Parses command-line arguments.

    Args:
        argv (list of str): List of command-line arguments.

    Returns:
        argparse.Namespace: Parsed arguments containing files, directories,
                            copyright_file, extensions and log_file.
    """
    parser = argparse.ArgumentParser(description="A script to check for copyright in files with specific extensions.")

    parser.add_argument(
        "-t",
        "--template-file",
        type=Path,
        required=True,
        help="Path to the template file",
    )

    parser.add_argument(
        "--exclusion-file",
        type=Path,
        required=False,
        help="Path to the file listing file paths excluded from the copyright check.",
    )

    parser.add_argument(
        "-c",
        "--config-file",
        type=Path,
        default=None,
        help="Path to the config file",
    )

    parser.add_argument("-v", "--verbose", action="store_true", help="Enable debug logging level")

    parser.add_argument(
        "-l",
        "--log-file",
        type=Path,
        default=None,
        help="Redirect logs from STDOUT to this file",
    )

    parser.add_argument(
        "-e",
        "--extensions",
        type=str,
        nargs="+",
        default=None,
        help="List of extensions to filter when searching for files, e.g., '.h .cpp'",
    )

    parser.add_argument(
        "--use_memory_map",
        action="store_true",
        help="Use memory map for reading content of files \
              (should be used reading gigabyte ranged files).",
    )

    parser.add_argument(
        "-f",
        "--fix",
        action="store_true",
        help="Fix missing copyright headers by inserting them",
    )

    parser.add_argument("--encoding", default="utf-8", help="File encoding (default: utf-8).")

    parser.add_argument(
        "--offset",
        dest="offset",
        type=int,
        default=0,
        help="Force this many characters (plus any trailing blank lines) at the start of the "
        "file to be treated as a recognized preamble (e.g. a shebang), overriding "
        "auto-detection. Character-based, not byte-based. Default: auto-detect (0).",
    )

    parser.add_argument(
        "--remove-offset",
        dest="remove_offset",
        type=int,
        default=0,
        help="Offset to remove old header from beginning of the file \
             (supported only with --fix mode)",
    )

    parser.add_argument(
        "--force",
        dest="force",
        action="store_true",
        help="With --fix, also rewrite WRONG_FORMAT/MISPLACED_AND_WRONG_FORMAT headers whose "
        "similarity to the template is below HEADER_SIMILARITY_THRESHOLD (normally left "
        "untouched since they may be a genuinely different license text). Never affects "
        "DUPLICATE or LICENSE_MISMATCH (a header whose own SPDX identifier genuinely differs "
        "from the template's), which always require manual review. Ignored without --fix.",
    )

    parser.add_argument(
        "inputs",
        nargs="*",
        action=ParamFileAction,
        help="Files and/or directories to check. When omitted, the whole repository (per 'git ls-files') is checked.",
    )

    parser.add_argument(
        "--modified-only",
        dest="modified_only",
        action="store_true",
        help="Only check files that differ from HEAD (staged and/or unstaged), "
        "e.g. for a fast, incremental pre-commit run. Takes precedence "
        "over 'inputs' (including any 'srcs' pathspec baked into a "
        "Bazel target's args), so the same whole-repo target used by CI "
        "can be reused for incremental, modified-files-only runs.",
    )

    return parser.parse_args(argv)


def main(argv=None):
    """
    Entry point for processing files to check for the presence of required copyright text.

    This function parses command-line arguments, configures logging, loads copyright templates,
    collects input files based on provided criteria, and checks each file for the required
    copyright text.

    Args:
        argv (list, optional): List of command-line arguments.
                               If `None`, defaults to `sys.argv[1:]`.

    Returns:
        int: ``0`` if all files are compliant, ``1`` if violations were found,
             ``2`` if the tool failed to run (e.g. unreadable inputs).
    """
    try:
        args = parse_arguments(argv if argv is not None else sys.argv[1:])
    except (IOError, OSError) as err:
        LOGGER.error("Failed to parse arguments: %s", err)
        return 2
    configure_logging(args.log_file, args.verbose)

    try:
        templates = load_templates(args.template_file)
    except IOError as err:
        LOGGER.error("Failed to load copyright text: %s", err)
        return 2

    if args.config_file:
        try:
            get_author_from_config(args.config_file)
        except (IOError, OSError, ValueError) as err:
            LOGGER.error("Failed to load config file: %s", err)
            return 2

    exclusion = []
    exclusion_valid = True
    workspace = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if args.exclusion_file:
        try:
            exclusion, exclusion_valid = load_exclusion(args.exclusion_file, workspace)
        except IOError as err:
            LOGGER.error("Failed to load exclusion list: %s", err)
            return 2

    # When invoked via ``bazel run`` the process runs inside the runfiles tree, but
    # the files to check live in the user's workspace. ``git ls-files``/``git diff``
    # (used by ``collect_inputs``/``list_modified_files``) are run from
    # ``BUILD_WORKSPACE_DIRECTORY`` so the checker operates on the real source
    # tree, honoring ``.gitignore`` (which excludes the ``bazel-*`` convenience
    # symlinks) and reaching nested Bazel packages.
    try:
        if args.modified_only:
            base_dir = workspace or Path.cwd()
            files = [
                f
                for f in list_modified_files(base_dir)
                if not f.is_symlink() and f.is_file() and _matches_extension(f, args.extensions)
            ]
        else:
            files = collect_inputs(args.inputs, args.extensions, workspace)
    except (IOError, subprocess.SubprocessError) as err:
        LOGGER.error("Failed to collect input files: %s", err)
        return 2

    if not files:
        LOGGER.warning("No files matched the configured extensions/inputs; nothing to check.")

    LOGGER.debug("Running check on files: %s", files)

    if args.fix and args.remove_offset:
        LOGGER.info("%s!------DANGER ZONE------!%s", COLORS["RED"], COLORS["ENDC"])
        LOGGER.info("Remove offset set! This can REMOVE parts of source files!")
        LOGGER.info("Use ONLY if invalid copyright header is present that needs to be removed!")
        LOGGER.info("%s!-----------------------!%s", COLORS["RED"], COLORS["ENDC"])

    if args.fix and args.force:
        LOGGER.info("%s!------DANGER ZONE------!%s", COLORS["RED"], COLORS["ENDC"])
        LOGGER.info("Force set! Headers with low similarity to the template will be overwritten too!")
        LOGGER.info("Review the diff afterwards -- this can rewrite genuinely different license text!")
        LOGGER.info("%s!-----------------------!%s", COLORS["RED"], COLORS["ENDC"])

    results = process_files(
        files,
        templates,
        args.fix,
        exclusion,
        args.config_file,
        args.use_memory_map,
        args.encoding,
        args.offset,
        args.remove_offset,
        args.force,
    )
    total_missing = results["missing"]
    total_misplaced = results["misplaced"]
    total_wrong_format = results["wrong_format"]
    total_duplicates = results["duplicate"]
    total_license_mismatches = results["license_mismatch"]
    total_fixes = results["fixed"]
    total_violations = (
        total_missing + total_misplaced + total_wrong_format + total_duplicates + total_license_mismatches
    )

    LOGGER.info("=" * 64)
    LOGGER.info("Process completed.")
    LOGGER.info(
        "Total files missing a copyright header: %s%d%s",
        COLORS["RED"] if total_missing > 0 else COLORS["GREEN"],
        total_missing,
        COLORS["ENDC"],
    )
    LOGGER.info(
        "Total files with a misplaced (but correctly formatted) header: %s%d%s",
        COLORS["RED"] if total_misplaced > 0 else COLORS["GREEN"],
        total_misplaced,
        COLORS["ENDC"],
    )
    LOGGER.info(
        "Total files with a wrong-format header: %s%d%s",
        COLORS["RED"] if total_wrong_format > 0 else COLORS["GREEN"],
        total_wrong_format,
        COLORS["ENDC"],
    )
    LOGGER.info(
        "Total files with duplicate copyright: %s%d%s",
        COLORS["RED"] if total_duplicates > 0 else COLORS["GREEN"],
        total_duplicates,
        COLORS["ENDC"],
    )
    LOGGER.info(
        "Total files with a license mismatch: %s%d%s",
        COLORS["RED"] if total_license_mismatches > 0 else COLORS["GREEN"],
        total_license_mismatches,
        COLORS["ENDC"],
    )
    if not exclusion_valid:
        LOGGER.info("The exclusion file contains paths that do not exist.")
    if args.fix:
        total_not_fixed = total_violations - total_fixes
        LOGGER.info(
            "Total files that were fixed: %s%d%s",
            COLORS["GREEN"],
            total_fixes,
            COLORS["ENDC"],
        )
        LOGGER.info(
            "Total files that were NOT fixed: %s%d%s",
            COLORS["RED"] if total_not_fixed > 0 else COLORS["GREEN"],
            total_not_fixed,
            COLORS["ENDC"],
        )
    LOGGER.info("=" * 64)

    return 0 if (total_violations == 0 and exclusion_valid) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
