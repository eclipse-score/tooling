#!/usr/bin/env python3
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

"""Merge multiple Sphinx HTML output directories.

This script merges Sphinx HTML documentation from multiple modules into a single
output directory. It copies the main module's HTML as-is, then copies every
transitively required module's *own* (unmerged) HTML into a subdirectory --
flat, one level deep, one copy per module regardless of how many dependency
paths reach it. sphinx_module.bzl is responsible for passing --dep as the
transitive closure (SphinxModuleInfo.transitive_modules), with each --dep
pointing at that module's own_html_dir rather than its (possibly
further-merged) html_dir -- this script has no way to tell the two apart and
would duplicate a diamond dependency if handed the latter.

Usage:
    sphinx_html_merge.py --output OUTPUT_DIR --main MAIN_HTML_DIR [--dep NAME:PATH ...]
"""

import argparse
import logging
import re
import shutil
import sys
from pathlib import Path

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}

# Element BODIES whose contents must never be touched by the link-rewriting
# regexes below: literal code samples (<pre>, <code>) and script bodies can
# legitimately contain the text `href="..."` / `src="..."` (e.g. an example
# snippet showing how to write a link) without it being an actual
# navigational attribute. Only the body (group 3) is protected — the opening
# tag itself (group 1) is left live, since a real `<script src="...">` has
# its src attribute *in the tag*, not the body, and must still be rewritten.
_PROTECTED_BLOCK_PATTERN = re.compile(
    r"(<(pre|code|script)\b[^>]*>)(.*?)(</\2>)",
    re.IGNORECASE | re.DOTALL,
)
_PROTECTED_PLACEHOLDER = "\x00PROTECTED_BLOCK_{}\x00"


def _protect_blocks(content):
    """Replace <pre>/<code>/<script> BODIES with placeholders.

    The opening/closing tags are left in place so link-rewriting regexes can
    still rewrite a real `<script src="...">` / `<code src="...">` attribute;
    only the body text between the tags is hidden. Returns
    (content_with_placeholders, blocks) so _restore_blocks() can put the
    original, untouched body content back afterwards.
    """
    blocks = []

    def _stash(match):
        blocks.append(match.group(3))
        return match.group(1) + _PROTECTED_PLACEHOLDER.format(len(blocks) - 1) + match.group(4)

    return _PROTECTED_BLOCK_PATTERN.sub(_stash, content), blocks


def _restore_blocks(content, blocks):
    for i, block in enumerate(blocks):
        content = content.replace(_PROTECTED_PLACEHOLDER.format(i), block)
    return content


# Sphinx build-cache artifacts that must never reach the published site.
# .doctrees holds pickled BuildEnvironment state (absolute execroot paths,
# per-doc mtimes) — copying it is both wasted space and a hermeticity leak.
BUILD_ARTIFACT_DIRS = {".doctrees"}


def copy_html_files(src_dir, dst_dir, is_dependency=False, sibling_modules=None):
    """Copy HTML and related files from src to dst, with optional link fixing.

    Args:
        src_dir: Source HTML directory
        dst_dir: Destination directory
        is_dependency: Whether src_dir is a dependency module's HTML being placed
                     into a subdirectory of the merged site (as opposed to the
                     main module, which is copied as-is at the site root).
                     Dependencies have their own _static/_sphinx_design_static
                     dropped (the merged site uses one shared _static/ at the
                     root) and their internal links rewritten for the new
                     nesting depth.
        sibling_modules: Set of other module directory names to rewrite intra-site
                        links for (e.g. href="other_module/..." needs a "../" prefix
                        added for the new nesting depth). Only meaningful when
                        is_dependency is True. src_dir is always a module's own,
                        unmerged HTML now (never another module's already-merged
                        tree), so there is nothing nested under it to skip.
    """
    src_path = Path(src_dir)
    dst_path = Path(dst_dir)

    if not src_path.exists():
        logging.warning("Source directory does not exist: %s", src_dir)
        return

    dst_path.mkdir(parents=True, exist_ok=True)

    sibling_modules = sibling_modules or set()

    # Prepare regex pattern for sibling-module link fixing, if needed.
    module_pattern = None
    if sibling_modules:
        module_pattern = re.compile(
            r'((?:href|src)=")(' + "|".join(re.escape(mod) for mod in sibling_modules) + r")/",
            re.IGNORECASE,
        )
    static_pattern = re.compile(r'((?:href|src)=")(\.\./)*(_static|_sphinx_design_static)/', re.IGNORECASE)

    def process_file(src_file, dst_file, relative_path):
        """Read, optionally modify, and write a file."""
        if src_file.suffix == ".html" and is_dependency:
            # Read, modify, and write HTML files
            try:
                content = src_file.read_text(encoding="utf-8")

                # Shield <pre>/<code>/<script> bodies before running the
                # link-rewriting regexes below, so example text or JS string
                # literals that happen to contain `href="..."` / `src="..."`
                # is never mistaken for a real attribute.
                content, protected_blocks = _protect_blocks(content)

                # Both rewrites below must agree on how many directory levels
                # this page now sits below the merged site root, so compute
                # the prefix once and share it.
                depth = len(relative_path.parents) - 1
                parent_prefix = "../" * (depth + 1)

                if module_pattern is not None:

                    def replace_module(match):
                        return f"{match.group(1)}{parent_prefix}{match.group(2)}/"

                    content = module_pattern.sub(replace_module, content)

                def replace_static(match):
                    return f"{match.group(1)}{parent_prefix}{match.group(3)}/"

                modified_content = static_pattern.sub(replace_static, content)
                modified_content = _restore_blocks(modified_content, protected_blocks)

                # Write modified content
                dst_file.parent.mkdir(parents=True, exist_ok=True)
                dst_file.write_text(modified_content, encoding="utf-8")
            except Exception as e:
                logging.warning("Failed to process %s: %s", src_file, e)
                # Fallback to regular copy on error
                shutil.copy2(src_file, dst_file)
        else:
            # Regular copy for non-HTML files
            dst_file.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src_file, dst_file)

    def copy_tree(src, dst, rel_path):
        """Recursively copy directory tree with processing."""
        for item in src.iterdir():
            rel_item = rel_path / item.name
            dst_item = dst / item.name

            if item.is_file():
                process_file(item, dst_item, rel_item)
            elif item.is_dir():
                # Never publish Sphinx's own build cache.
                if item.name in BUILD_ARTIFACT_DIRS:
                    continue
                # Dependencies use the merged site's shared _static/ instead
                # of their own.
                if is_dependency and item.name in (
                    "_static",
                    "_sphinx_design_static",
                ):
                    continue

                dst_item.mkdir(parents=True, exist_ok=True)
                copy_tree(item, dst_item, rel_item)

    # Start copying from root
    copy_tree(src_path, dst_path, Path("."))


def merge_html_dirs(output_dir, main_html_dir, dependencies, extra_static=None):
    """Merge HTML directories.

    Args:
        output_dir: Target output directory
        main_html_dir: Main module's HTML directory to copy as-is
        dependencies: List of (name, path) tuples for dependency modules
        extra_static: List of (src_file, dest_subpath) tuples for extra files to
                      place in output/_static/.  These are copied AFTER the main
                      HTML so they overwrite any theme-provided files if needed.
    """
    output_path = Path(output_dir)

    # First, copy the main HTML directory
    logging.info("Copying main HTML from %s to %s", main_html_dir, output_dir)
    copy_html_files(main_html_dir, output_dir, is_dependency=False)

    # Copy any extra static files into output/_static/
    for src_file, dest_subpath in extra_static or []:
        dst = output_path / "_static" / dest_subpath
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src_file, dst)
        logging.info("Copied extra static %s → _static/%s", src_file, dest_subpath)

    # Collect all dependency names for intra-site link rewriting.
    dep_names = [name for name, _ in dependencies]

    # Then copy each dependency into a subdirectory with link fixing
    for dep_name, dep_html_dir in dependencies:
        dep_output = output_path / dep_name
        logging.info("Copying dependency %s from %s to %s", dep_name, dep_html_dir, dep_output)
        # Other modules in this merge, to rewrite intra-site links for.
        sibling_modules = set(n for n in dep_names if n != dep_name)
        copy_html_files(
            dep_html_dir,
            dep_output,
            is_dependency=True,
            sibling_modules=sibling_modules,
        )


def main():
    parser = argparse.ArgumentParser(description="Merge Sphinx HTML documentation directories")
    parser.add_argument("--output", required=True, help="Output directory for merged HTML")
    parser.add_argument("--main", required=True, help="Main HTML directory to copy")
    parser.add_argument(
        "--dep",
        action="append",
        default=[],
        metavar="NAME:PATH",
        help="Dependency HTML directory in format NAME:PATH",
    )
    parser.add_argument(
        "--extra-static",
        action="append",
        default=[],
        metavar="SRC:SUBPATH",
        help="Extra file to place in output/_static/.  Format: SRC_FILE:DEST_SUBPATH",
    )
    parser.add_argument(
        "--log-level",
        choices=["error", "warn", "info", "debug"],
        default="warn",
        dest="log_level",
        help="Log level for tool output (default: warn).",
    )

    args = parser.parse_args()
    logging.basicConfig(level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s")

    # Parse dependencies
    dependencies = []
    for dep_spec in args.dep:
        if ":" not in dep_spec:
            logging.error("Invalid dependency format '%s', expected NAME:PATH", dep_spec)
            return 1

        name, path = dep_spec.split(":", 1)
        dependencies.append((name, path))

    # Parse extra static files
    extra_static = []
    for spec in args.extra_static:
        if ":" not in spec:
            logging.error("Invalid --extra-static format '%s', expected SRC:SUBPATH", spec)
            return 1
        src, subpath = spec.split(":", 1)
        extra_static.append((src, subpath))

    # Merge the HTML directories
    merge_html_dirs(args.output, args.main, dependencies, extra_static=extra_static)

    logging.info("Successfully merged HTML into %s", args.output)
    return 0


if __name__ == "__main__":
    sys.exit(main())
