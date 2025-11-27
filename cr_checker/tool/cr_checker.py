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
"""The tool for checking if artifacts have proper copyright."""

import argparse
import json
import logging
import mmap
import os
import re
import shutil
import sys
import tempfile
from datetime import datetime
from pathlib import Path

BYTES_TO_READ = 4 * 1024
DEFAULT_AUTHOR = "Contributors to the Eclipse Foundation"

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
                add_template_for_extensions(
                    templates, current_extensions, template_for_extensions
                )

                template_for_extensions = ""

                extensions = stripped_line[1:-1].split(",")
                current_extensions = [ext.strip() for ext in extensions]
                LOGGER.debug(current_extensions)
            else:
                template_for_extensions += line

        add_template_for_extensions(
            templates, current_extensions, template_for_extensions
        )

    LOGGER.debug(templates)
    return templates


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


def detect_shebang_offset(path, encoding):
    """
    Detects if a file starts with a shebang (#!) and returns the byte offset
    to skip it (length of the first line including newline).

    Args:
        path (Path): A `pathlib.Path` object pointing to the file.
        encoding (str): Encoding type to use when reading the file.

    Returns:
        int: The byte length of the shebang line (including newline) if present,
             otherwise 0.
    """
    try:
        with open(path, "r", encoding=encoding) as handle:
            first_line = handle.readline()
            if first_line.startswith("#!"):
                # Calculate byte length of the first line
                byte_length = len(first_line.encode(encoding))
                while True:
                    next_char = handle.read(1)
                    if not next_char or next_char not in ("\n", "\r"):
                        break
                    byte_length += len(next_char.encode(encoding))
                LOGGER.debug(
                    "Detected shebang in %s with offset %d bytes", path, byte_length
                )
                return byte_length
    except (IOError, OSError) as err:
        LOGGER.debug("Could not detect shebang in %s: %s", path, err)
    return 0


def load_text_from_file(path, header_length, encoding, offset):
    """
    Reads the first portion of a file, up to `header_length` characters
    plus an additional offset if provided.

    Args:
        path (Path): A `pathlib.Path` object pointing to the file.
        header_length (int): Number of characters to read for the header.
        encoding (str): Encoding type to use when reading the file.
        offset (int): Additional number of characters to read beyond
                      `header_length`, typically used to account for extra
                      lines (such as a shebang) before the header.

    Returns:
        str: The portion of the file read, which should contain the header if present,
             including any extra characters specified by `offset`.
    """
    total_length = header_length + offset
    LOGGER.debug(
        "Reading first %d characters from file: %s [%s]", total_length, path, encoding
    )
    with open(path, "r", encoding=encoding) as handle:
        content = handle.read(total_length)
        return content[offset:] if offset else content


def load_text_from_file_with_mmap(path, header_length, encoding, offset):
    """
    Maps the file and reads only the first `header_length` bytes plus
    an additional offset if provided.

    Args:
        path (Path): A `pathlib.Path` object pointing to the file.
        header_length (int): Length of the header text to check.
        encoding (str): String for setting decoding type.
        offset (int): Additional number of characters to read beyond
                      `header_length`, typically used to account for extra
                      lines (such as a shebang) before the header.

    Returns:
        str: The portion of the file read, which should contain the header if present.
    """

    file_size = os.path.getsize(path)
    total_length = header_length + offset
    length = min(total_length, file_size)

    if not length:
        LOGGER.warning(
            "File %s is empty [length: %d]. Return empty string.", path, length
        )
        return ""

    LOGGER.debug("Memory mapping first %d bytes from file: %s", total_length, path)
    with open(path, "r", encoding=encoding) as handle:
        with mmap.mmap(handle.fileno(), length=length, access=mmap.ACCESS_READ) as fmap:
            return fmap[:length].decode(encoding)[offset:]


def has_copyright(path, template, use_mmap, encoding, offset, config=None):
    """
    Checks if the specified copyright text is present in the beginning of a file.

    Args:
        path (Path): A `pathlib.Path` object pointing to the file to check.
        template (str): The copyright text to search for at the beginning
                              of the file.
        use_mmap (bool): If True, uses memory-mapped file reading for efficient
                         large file handling.
        encoding (str): Encoding type to use when reading the file.
        offset (int): Additional number of characters to read beyond the length
                      of `copyright_text`, used to account for extra content
                      (such as a shebang) before the copyright text.
        config (Path): Path to the config JSON file where configuration
                variables are stored (e.g. years for copyright headers).

    Returns:
        bool: True if the file contains the copyright text, False if it is missing.

    Raises:
        IOError: If there is an error opening or reading the file.
    """

    load_text = load_text_from_file
    if use_mmap:
        load_text = load_text_from_file_with_mmap

    template_regex = convert_bre_to_regex(
        template.format(year=r"\\d\{4\}", author=r"\.\*")
    )

    if re.match(template_regex, load_text(path, BYTES_TO_READ, encoding, offset)):
        LOGGER.debug("File %s has copyright.", path)
        return True

    LOGGER.debug("File %s doesn't have copyright.", path)
    return False


def get_files_from_dir(directory, exts=None):
    """
    Finds files in the specified directories. Filters by extensions if provided.

    Args:
        dirs (list of str): List of directories to search for files.
        exts (list of str, optional): List of extensions to filter files.
                                      If None, all files are returned.

    Returns:
        list of str: List of file paths found in the directories.
    """
    collected_files = []
    LOGGER.debug("Getting files from directory: %s", directory)
    for path in directory.rglob("*"):
        if path.is_file() and path.stat().st_size != 0:
            if (
                exts is None
                or path.suffix[1:] in exts
                or (path.name == "BUILD" and "BUILD" in exts)
            ):
                collected_files.append(path)
    return collected_files


def collect_inputs(inputs, exts=None):
    """
    Collects files from a list of input paths, optionally filtering by file extensions.

    Args:
        inputs (list): A list of paths to files or directories.
                       If a directory is provided, all files within it are added to the output.
        exts (list, optional): A list of file extensions to filter by (e.g., ['.py', '.txt']).
                               Only files with these extensions will be included if specified.

    Returns:
        list: A list of file paths collected from the input paths, filtered by the given extensions.
              If an input is neither a file nor a directory, it is skipped with a warning.

    Logs:
        Logs messages at the DEBUG level, detailing processing of directories and files,
        and warns if an invalid input path is encountered.
    """
    all_files = []
    LOGGER.debug("Extensions: %s", exts)
    for i in inputs:
        item = Path(i)
        if item.is_dir():
            LOGGER.debug("Processing directory: %s", item)
            all_files.extend(get_files_from_dir(item, exts))
        elif item.is_file() and (
            exts is None
            or item.suffix[1:] in exts
            or (item.name == "BUILD" and "BUILD" in exts)
        ):
            LOGGER.debug("Processing file: %s", item)
            all_files.append(item)
        else:
            LOGGER.warning("Skipped (input is not a valid file or directory): %s", item)
    return all_files


def create_temp_file(path, encoding):
    """
    Creates a temporary file with the provided content.

    Args:
        path (str): The path of file to write the content to the temporary file.
        encoding (str, optional): Encoding type to use when writing the file.

    Returns:
        str: The path to the temporary file created.
    """
    with tempfile.NamedTemporaryFile(mode="w", encoding=encoding, delete=False) as temp:
        with open(path, "r", encoding=encoding) as handle:
            for chunk in iter(lambda: handle.read(4096), ""):
                temp.write(chunk)
    return temp.name


def remove_old_header(file_path, encoding, num_of_chars):
    """
    Removes the first `num_of_chars` characters from a file and updates it in-place.

    Args:
        file_path (str): Path to the file to be modified.
        encoding (str): Encoding used to read and write the file.
        num_of_chars (int): Number of characters to remove from the beginning of the file.

    Raises:
        IOError: If there is an issue reading or writing the file.
        ValueError: If `num_of_chars` is negative.
    """
    with open(file_path, "r", encoding=encoding) as file:
        file.seek(num_of_chars)
        with tempfile.NamedTemporaryFile(
            "w", delete=False, encoding=encoding
        ) as temp_file:
            shutil.copyfileobj(file, temp_file)
    shutil.move(temp_file.name, file_path)


def fix_copyright(path, copyright_text, encoding, offset, config=None):
    """
    Inserts a copyright header into the specified file, ensuring that existing
    content is preserved according to the provided offset.

    Args:
        path (str): The path to the file that needs the copyright header.
        copyright_text (str): The copyright text to be added.
        encoding (str): The character encoding used to read and write the file.
        offset (int): The number of bytes to preserve at the top of the file.
                      If 0, the first line is overwritten unless it's empty.
                      For non-zero offsets, ensures the correct number of bytes
                      are preserved.
        config (Path): Path to the config JSON file where configuration
                variables are stored (e.g. years for copyright headers).
    """

    temporary_file = create_temp_file(path, encoding)

    with open(temporary_file, "r", encoding=encoding) as temp:
        first_line = temp.readline()
        byte_array = len(first_line.encode(encoding))

        if offset > 0 and offset != byte_array:
            LOGGER.error("Invalid offset value: %d, expected: %d", offset, byte_array)
            return

        with open(path, "w", encoding=encoding) as handle:
            temp.seek(0)
            if offset > 0:
                handle.write(first_line)
                temp.seek(offset)
            handle.write(
                copyright_text.format(
                    year=datetime.now().year, author=get_author_from_config(config)
                )
            )
            for chunk in iter(lambda: temp.read(4096), ""):
                handle.write(chunk)
    LOGGER.info("Fixed missing header in: %s", path)


def process_files(
    files,
    templates,
    fix,
    config=None,
    use_mmap=False,
    encoding="utf-8",
    offset=0,
    remove_offset=0,
):  # pylint: disable=too-many-arguments
    """
    Processes a list of files to check for the presence of copyright text.

    Args:
        files (list): A list of file paths to check.
        templates (dict): A dictionary where keys are file extensions
                          (e.g., '.py', '.txt') and values are strings or patterns
                          representing the required copyright text.
        config (Path): Path to the config JSON file where configuration
                       variables are stored (e.g. years for copyright headers).
        use_mmap (bool): Flag for using mmap function for reading files
                         (instead of standard option).
        encoding (str): Encoding type to use when reading the file.
        offset (int): Additional number of characters to read beyond the length
                      of `copyright_text`, used to account for extra content
                      (such as a shebang) before the copyright text.
        remove_offset(int): Flag for removing old header from source files
                             (before applying the new one) in number of chars.

    Returns:
        int: The number of files that do not contain the required copyright text.
    """
    results = {"no_copyright": 0, "fixed": 0}
    for item in files:
        name = Path(item).name
        key = name if name == "BUILD" else Path(item).suffix[1:]
        if key not in templates.keys():
            logging.debug(
                "Skipped (no configuration for selected file extension): %s", item
            )
            continue

        # Automatically detect shebang and use its offset if no manual offset provided
        shebang_offset = detect_shebang_offset(item, encoding)
        effective_offset = offset + shebang_offset if offset == 0 else offset

        if not has_copyright(
            item, templates[key], use_mmap, encoding, effective_offset, config
        ):
            if fix:
                if remove_offset:
                    remove_old_header(item, encoding, remove_offset)
                fix_copyright(item, templates[key], encoding, effective_offset, config)
                results["no_copyright"] += 1
                results["fixed"] += 1
            else:
                LOGGER.error(
                    "Missing copyright header in: %s, use --fix to introduce it", item
                )
                results["no_copyright"] += 1
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
    parser = argparse.ArgumentParser(
        description="A script to check for copyright in files with specific extensions."
    )

    parser.add_argument(
        "-t",
        "--template-file",
        type=Path,
        required=True,
        help="Path to the template file",
    )

    parser.add_argument(
        "-c",
        "--config-file",
        type=Path,
        default=None,
        help="Path to the config file",
    )

    parser.add_argument(
        "-v", "--verbose", action="store_true", help="Enable debug logging level"
    )

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

    parser.add_argument(
        "--encoding", default="utf-8", help="File encoding (default: utf-8)."
    )

    parser.add_argument(
        "--offset",
        dest="offset",
        type=int,
        default=0,
        help="Additional length offset to account for characters like a shebang (default is 0)",
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
        "inputs",
        nargs="+",
        action=ParamFileAction,
        help="Directories and/or files to parse.",
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
        int: Error code if an IOError occurs during loading templates or collecting input files;
        otherwise, returns 0 as success.
    """
    args = parse_arguments(argv if argv is not None else sys.argv[1:])
    configure_logging(args.log_file, args.verbose)

    try:
        templates = load_templates(args.template_file)
    except IOError as err:
        LOGGER.error("Failed to load copyright text: %s", err)
        return err.errno

    try:
        files = collect_inputs(args.inputs, args.extensions)
    except IOError as err:
        LOGGER.error("Failed to process file %s with error", err.filename)
        return err.errno

    LOGGER.debug("Running check on files: %s", files)

    if args.fix and args.remove_offset:
        LOGGER.info("%s!------DANGER ZONE------!%s", COLORS["RED"], COLORS["ENDC"])
        LOGGER.info("Remove offset set! This can REMOVE parts of source files!")
        LOGGER.info(
            "Use ONLY if invalid copyright header is present that needs to be removed!"
        )
        LOGGER.info("%s!-----------------------!%s", COLORS["RED"], COLORS["ENDC"])

    results = process_files(
        files,
        templates,
        args.fix,
        args.config_file,
        args.use_memory_map,
        args.encoding,
        args.offset,
        args.remove_offset,
    )
    total_no = results["no_copyright"]
    total_fixes = results["fixed"]

    LOGGER.info("=" * 64)
    LOGGER.info("Process completed.")
    LOGGER.info(
        "Total files without copyright: %s%d%s",
        COLORS["RED"] if total_no > 0 else COLORS["GREEN"],
        total_no,
        COLORS["ENDC"],
    )
    if args.fix:
        total_not_fixed = total_no - total_fixes
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

    return 0 if total_no == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
