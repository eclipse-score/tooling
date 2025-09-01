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
"""The tool for converting requirements.txt into dash checker format"""

import argparse
import logging
import re
import sys
from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib

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
    A custom logging formatter to add color to log level names
    based on the logging level.

    The `ColoredFormatter` class extends `logging.Formatter` and overrides the `format`
    method to add color codes to the log level name (e.g., `INFO`, `WARNING`, `ERROR`)
    based on a predefined color mapping in `LOGGER_COLORS`. This color coding helps in
    visually distinguishing log messages by severity.

    Attributes:
        LOGGER_COLORS (dict): A dictionary mapping log level names
                              (e.g., "INFO", "ERROR") to their respective color codes.
        COLORS (dict): A dictionary of terminal color codes, including an "ENDC" key
                       to reset colors after the level name.

    Methods:
        format(record): Adds color to the `levelname` attribute of the log record and
                        then formats the record as per the superclass `Formatter`.
    """

    def format(self, record):
        log_color = LOGGER_COLORS.get(record.levelname, "")
        record.levelname = f"{log_color}{record.levelname}:{COLORS['ENDC']}"
        return super().format(record)


def configure_logging(log_file_path: Path | None = None, verbose: bool = False) -> None:
    """
    Configures the logging settings for the application.

    Args:
        log_file_path (Path, optional): The file path where logs should be written.
            If not provided, logs are written to the console. Defaults to `None`.
        verbose (bool, optional): A flag that determines the logging level.
            If `True`, sets the logging level to DEBUG; if `False`, sets it to INFO.
            Defaults to `False`.

    Returns:
        None: This function does not return any value, it only configures logging.

    Notes:
        - If `log_file_path` is provided, the log messages will be saved
          to the specified file.
        - If `verbose` is `True`, detailed logs (DEBUG level) will be captured;
          otherwise, less detailed logs (INFO level) will be captured.
        - The default logging format is:
          `%(asctime)s - %(name)s - %(levelname)s - %(message)s`.
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


def format_line(
    line: str, regex: str = r"([a-zA-Z0-9_-]+)==([a-zA-Z0-9.\-_]+)"
) -> str | None:
    """
    Formats a line of text by matching a specified regex pattern and
    extracting components.

    Args:
        line (str): The input string to be processed.
        regex (str, optional): A regular expression pattern to match the input line.

    Returns:
        Optional[str]: A formatted string based on the regex match
        if the pattern is found, or None if no match is found.

    Notes:
        - The function uses the provided regex to capture two components
          from the input line: the package name and its version.
        - The formatted string follows the pattern "pypi/pypi/-/{package}/{version}".
        - If the regex does not match, None is returned.
    """

    ret = None
    match = re.match(f"{regex}", line)

    if match:
        package, version = match.groups()
        ret = f"pypi/pypi/-/{package}/{version}"

    return ret


def convert_to_dash_format(input_file: Path, output_file: Path):
    """
    Converts the content of an input to a "dash format" and writes output file.

    The exact transformation applied to the content is assumed to replace specific
    patterns or structures with a dash-separated format, although the details depend
    on the implementation.

    Args:
        input_file (Path): Path to the input file containing the original content.
        output_file (Path): Path to the output file where the converted content
                            will be written.

    """
    encoding = "utf-8"
    with (
        open(input_file, encoding=encoding) as infile,
        open(output_file, "w", encoding=encoding) as outfile,
    ):
        for line in infile:
            formatted_line = format_line(line.strip())
            if formatted_line:
                outfile.write(formatted_line + "\n")


def convert_cargo_to_dash_format(
    input_file: Path,
    output_file: Path,
    skip_source_filter: bool = False,
    filter_keywords: list[str] | None = None,
) -> int:
    """
    Converts a Cargo.lock file into dash format.

    This function parses the TOML-formatted Cargo.lock file, iterates over the package entries,
    and writes each package's name and version in the format:
        cargo/cargo/-/{name}/{version}
    """
    encoding = "utf-8"

    # Set default keywords if none provided
    if filter_keywords is None:
        filter_keywords = ["dummy-source-keyword"]

    try:
        with open(input_file, "rb") as infile:
            cargo_data = tomllib.load(infile)
    except Exception as e:
        LOGGER.error(f"Error parsing Cargo.lock file: {e}")
        return 1

    packages = cargo_data.get("package", [])

    if not packages:
        LOGGER.warning("No packages found in Cargo.lock.")

    with open(output_file, "w", encoding=encoding) as outfile:
        for pkg in packages:
            if not skip_source_filter:
                # If the package has no source, skip it, its developed internally
                if "source" not in pkg:
                    continue

                # Filter out packages with any of the specified keywords in the source
                source = pkg.get("source", "")
                if any(keyword in source for keyword in filter_keywords):
                    continue

            name = pkg.get("name")
            version = pkg.get("version")
            if name and version:
                # Remove leading 'v' if present
                version = version.lstrip("v")
                line = f"crate/cratesio/-/{name}/{version}"
                LOGGER.debug(f"Extracted package: {line}")
                outfile.write(line + "\n")

    LOGGER.info(
        f"Successfully converted {len(packages)} packages from Cargo.lock to {output_file}"
    )

    return 0


def parse_arguments(argv: list[str]) -> argparse.Namespace:
    """
    Parses command-line arguments passed to the script.

    Args:
        argv (list[str]): A list of command-line arguments, typically `sys.argv[1:]`.

    Returns:
        argparse.Namespace: An object containing the parsed arguments as attributes.

    Notes:
        - This function expects an `argparse.ArgumentParser` to be configured with
          the required arguments.
          If `argv` is not provided, it defaults to an empty list.
        - Use the `argparse.Namespace` object to access parsed arguments by their names.
    """

    parser = argparse.ArgumentParser(
        description="The tool for converting requirements.txt into dash \
                     checker format."
    )

    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Enable debug logging level",
    )

    parser.add_argument(
        "-l",
        "--log-file",
        type=Path,
        default=None,
        help="Redirect logs from STDOUT to this file",
    )

    parser.add_argument(
        "-i",
        "--input",
        type=Path,
        required=True,
        help="Path to the requirement.txt file",
    )

    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        required=True,
        help="Path to the formatted_list.txt file",
    )

    parser.add_argument(
        "-t",
        "--type",
        choices=["requirements", "cargo"],
        default="requirements",
        help="Type of input file: 'requirements' for requirements.txt or 'cargo' for Cargo.lock (default: requirements)",
    )

    parser.add_argument(
        "--skip-source-filter",
        action="store_true",
        help="Skip filtering out packages based on source (no source check, no keyword filtering)",
    )

    parser.add_argument(
        "--filter-keywords",
        nargs="*",
        default=["dummy-source-keyword"],
        help="Keywords to filter out from package sources. Packages containing any of these keywords in their source will be excluded.",
    )

    return parser.parse_args(argv)


def main(argv: list[str] | None) -> int:
    """
    The main entry point of the script.

    This function parses command-line arguments, configures logging, and dispatches the appropriate
    conversion function based on the input file type. Supported file types include:
      - 'requirements': for processing requirements.txt files.
      - 'cargo': for processing Cargo.lock files.
    The selected conversion function reads the input file, transforms its content into the Eclipse dash
    format, and writes the results to the specified output file.

    Args:
        argv (list[str], optional): A list of command-line arguments. If not provided, defaults to
            None, in which case sys.argv[1:] is used.

    Returns:
        int: An exit code where 0 indicates successful execution, and any non-zero value indicates
            an error occurred during processing.

    Notes:
        - This function is typically invoked in the 'if __name__ == "__main__":' block.
        - It orchestrates argument parsing, logging configuration, file conversion, and error handling.
        - Conversion behavior depends on the '--type' argument:
            * 'requirements' (default): Converts a requirements.txt file.
            * 'cargo': Converts a Cargo.lock file using TOML parsing.
        - Errors during file reading, parsing, or conversion are logged appropriately, and a non-zero
          exit code is returned to signal failure.
    """
    args = parse_arguments(argv if argv is not None else sys.argv[1:])
    configure_logging(args.log_file, args.verbose)
    if args.type == "cargo":
        ret = convert_cargo_to_dash_format(
            args.input, args.output, args.skip_source_filter, args.filter_keywords
        )
    else:
        ret = convert_to_dash_format(args.input, args.output)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
