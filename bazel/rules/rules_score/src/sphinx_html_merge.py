#!/usr/bin/env python3
"""Merge multiple Sphinx HTML output directories.

This script merges Sphinx HTML documentation from multiple modules into a single
output directory. It copies the main module's HTML as-is, and then copies each
dependency module's HTML into a subdirectory, excluding nested module directories
to avoid duplication.

Usage:
    sphinx_html_merge.py --output OUTPUT_DIR --main MAIN_HTML_DIR [--dep NAME:PATH ...]
"""

import argparse
import os
import re
import shutil
import sys
from pathlib import Path


# Standard Sphinx directories that should be copied
# Note: _static and _sphinx_design_static are excluded for dependencies to avoid duplication
SPHINX_DIRS = {"_sources", ".doctrees"}


def copy_html_files(src_dir, dst_dir, exclude_module_dirs=None, sibling_modules=None):
    """Copy HTML and related files from src to dst, with optional link fixing.

    Args:
        src_dir: Source HTML directory
        dst_dir: Destination directory
        exclude_module_dirs: Set of module directory names to skip (to avoid copying nested modules).
                           If None, copy everything.
        sibling_modules: Set of sibling module names for fixing links in HTML files.
                        If None, no link fixing is performed.
    """
    src_path = Path(src_dir)
    dst_path = Path(dst_dir)

    if not src_path.exists():
        print(f"Warning: Source directory does not exist: {src_dir}", file=sys.stderr)
        return

    dst_path.mkdir(parents=True, exist_ok=True)

    if exclude_module_dirs is None:
        exclude_module_dirs = set()

    # Prepare regex patterns for link fixing if needed
    module_pattern = None
    static_pattern = None
    if sibling_modules:
        module_pattern = re.compile(
            r'((?:href|src)=")('
            + "|".join(re.escape(mod) for mod in sibling_modules)
            + r")/",
            re.IGNORECASE,
        )
        static_pattern = re.compile(
            r'((?:href|src)=")(\.\./)*(_static|_sphinx_design_static)/', re.IGNORECASE
        )

    def process_file(src_file, dst_file, relative_path):
        """Read, optionally modify, and write a file."""
        if src_file.suffix == ".html" and sibling_modules:
            # Read, modify, and write HTML files
            try:
                content = src_file.read_text(encoding="utf-8")

                # Replace module_name/ with ../module_name/
                modified_content = module_pattern.sub(r"\1../\2/", content)

                # Calculate depth for static file references
                depth = len(relative_path.parents) - 1
                parent_prefix = "../" * (depth + 1)

                def replace_static(match):
                    return f"{match.group(1)}{parent_prefix}{match.group(3)}/"

                modified_content = static_pattern.sub(replace_static, modified_content)

                # Write modified content
                dst_file.parent.mkdir(parents=True, exist_ok=True)
                dst_file.write_text(modified_content, encoding="utf-8")
            except Exception as e:
                print(f"Warning: Failed to process {src_file}: {e}", file=sys.stderr)
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
                # Skip excluded directories
                if item.name in exclude_module_dirs:
                    continue
                # Skip static dirs from dependencies
                if (
                    item.name in ("_static", "_sphinx_design_static")
                    and exclude_module_dirs
                ):
                    continue

                dst_item.mkdir(parents=True, exist_ok=True)
                copy_tree(item, dst_item, rel_item)

    # Start copying from root
    copy_tree(src_path, dst_path, Path("."))


def merge_html_dirs(output_dir, main_html_dir, dependencies):
    """Merge HTML directories.

    Args:
        output_dir: Target output directory
        main_html_dir: Main module's HTML directory to copy as-is
        dependencies: List of (name, path) tuples for dependency modules
    """
    output_path = Path(output_dir)

    # First, copy the main HTML directory
    print(f"Copying main HTML from {main_html_dir} to {output_dir}")
    copy_html_files(main_html_dir, output_dir)

    # Collect all dependency names for link fixing and exclusion
    dep_names = [name for name, _ in dependencies]

    # Then copy each dependency into a subdirectory with link fixing
    for dep_name, dep_html_dir in dependencies:
        dep_output = output_path / dep_name
        print(f"Copying dependency {dep_name} from {dep_html_dir} to {dep_output}")
        # Exclude other module directories to avoid nested modules
        # Remove current module from the list to get actual siblings to exclude
        sibling_modules = set(n for n in dep_names if n != dep_name)
        copy_html_files(
            dep_html_dir,
            dep_output,
            exclude_module_dirs=sibling_modules,
            sibling_modules=sibling_modules,
        )


def main():
    parser = argparse.ArgumentParser(
        description="Merge Sphinx HTML documentation directories"
    )
    parser.add_argument(
        "--output", required=True, help="Output directory for merged HTML"
    )
    parser.add_argument("--main", required=True, help="Main HTML directory to copy")
    parser.add_argument(
        "--dep",
        action="append",
        default=[],
        metavar="NAME:PATH",
        help="Dependency HTML directory in format NAME:PATH",
    )

    args = parser.parse_args()

    # Parse dependencies
    dependencies = []
    for dep_spec in args.dep:
        if ":" not in dep_spec:
            print(
                f"Error: Invalid dependency format '{dep_spec}', expected NAME:PATH",
                file=sys.stderr,
            )
            return 1

        name, path = dep_spec.split(":", 1)
        dependencies.append((name, path))

    # Merge the HTML directories
    merge_html_dirs(args.output, args.main, dependencies)

    print(f"Successfully merged HTML into {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
