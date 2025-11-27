# CopyRight Checker

`cr_checker.py` is a tool designed to check if files contain a specified copyright header. It provides configurable logging, color-coded console output, and can handle large file sets efficiently. The script supports reading configuration files for custom copyright templates and can utilize memory-mapped file reading for better performance with large files. Tool itself can also append copyright header at the beginning of file if flag `--fix` is used.

## Features

- Checks files for specified copyright headers based on file extensions.
- Configurable logging, including color-coded output for easy visibility of log levels.
- Supports parameter files for flexible input handling.
- Can use memory mapping for large file handling.
- Customizable file encoding and offset adjustments for header text positioning.
- Can append copyright headers.
- Can remove provided number of characters from beginning of the file.

## Requirements

- Python 3.6+
- `argparse`, `logging`, `os`, `sys`, `mmap`, `tempfile`, and `pathlib` (standard library modules)

## Installation

To use `cr_checker.py`, simply clone this repository.

## Usage

The script can be run from the command line with various options to customize its behavior:

```bash
python cr_checker.py -t <template_file> [options] <inputs>
```

### Arguments

- **-t**, **--template-file**: (Required) Path to the template file that defines the copyright text for each file extension.
- **-v**, **--verbose**: Enable debug-level logging.
- **-l**, **--log-file**: Path to a log file where logs will be saved. If not provided, logs will print to the console.
- **-e**, **--extensions**: List of file extensions to filter, e.g., -e .py .cpp.
- **--use_memory_map**: Use memory-mapped file reading for large files.
- **--encoding**: File encoding (default is utf-8).
- **--offset**: Additional offset for the header length to account for lines like a shebang.
- **--fix**: Setting script into fix mode where copyright header will be added to the files if it's missing from same.
- **--remove-offset**: Number of characters to remove before appending proper copyright header (works only with `--fix` option).
- **inputs**: (Required) Directories or files to parse, or a parameter file prefixed with @ that lists files or directories.

> NOTE: Option `--remove-offset` can have severe consequences if the offset is miscalculated. Use with **extreme caution**.

> NOTE: Setting directory as `.` will cause that tool removes your complete workspace! This is connected with how Bazel includes python into build. **DO NOT USE THIS OPTION UNLESS YOU'RE 100% SURE IN WHAT YOU'RE DOING**.

### Examples

```sh
python cr_checker.py -t templates.ini -e py cpp -v -l logs.txt my_random_file.cpp my_random_file.py

python cr_checker.py -t templates.ini -e py cpp --offset 24 --use_memory_map @files_to_check.txt

python cr_checker.py -t templates.ini -e py cpp --fix --offset 24 --use_memory_map @files_to_check.txt

```

#### A bit more about `--offset`

This mode is special that will enable tool to do advance search & replace copyright headers. For example, assuming that we have following python implementation:

```python
#!/usr/bin/env python3

import os
```

and we use following command:

```sh
python cr_checker.py -t templates.ini -e py cpp --fix --use_memory_map @files_to_check.txt
```

The result will be following:

```python
##################
# COPYRIGHT HEADER
##################
#!/usr/bin/env python3

import os
```

This is of course not what we want, and for that we use `--offset=24` where 24 is number of char + 1 (for new line char).
if we apply this arguments now on same file, the outcome is different.

```python
#!/usr/bin/env python3

##################
# COPYRIGHT HEADER
##################

import os
```

On another hand, `--offset` has also another role. Assuming that a header text in file is soo big that copyright header is in the middle of the file, with regular command, the tool will not detect copyright headed. With `--offset=<NUM>` we can tell the tool from where to start considering the search for
copyright header.

### Template File Format

The template file should be in INI format, with each section representing a file extension and a section specifying the copyright text.
The copyright text can use format expressions to match the year and the author.

Example templates.ini:

```ini
[py,sh]
# Copyright (c) {year} {author}

[cpp,c,hpp, h]
// Copyright (c) {year} {author}
```

## Exit Codes

- 0: All files contain the required copyright text.
- 1: Some files are missing the required copyright text.
- Other: Error encountered during file processing.

### Logging and Color-Coded Output

By default, logs are printed to the console in color-coded format to indicate log levels. You can redirect logs to a file using the -l option.

#### Log Colors

- DEBUG: Blue
- INFO: Green
- WARNING: Yellow
- ERROR: Red

## Bazel integration

### Copyright Checker Bazel Macro

To integrate copyright verification into your Bazel-based project, you can use the `copyright_checker` macro. This macro allows you to check source files for compliance with a specified copyright template and configuration. Additionally, it can automatically apply fixes when necessary.

#### Usage

```python
load("@score_cr_checker//cr_checker:def.bzl", "copyright_checker")
copyright_checker(
    name = "copyright_check",
    srcs = glob(["src/**/*.cpp", "src/**/*.h"]),
    config = "@score_cr_checker//tools/cr_checker/resources:config",
    template = "@score_cr_checker//tools/cr_checker/resources:templates",
    visibility = ["//visibility:public"],
)
```

#### Parameters

- **name**: Unique identifier for the rule.
- **srcs**: List of source files to check.
- **visibility**: Defines which targets can access this rule.
- **template**: Path to the copyright header template.
- **config**: Path to the project-specific configuration.
- **extensions** (optional): List of file extensions to filter files. Defaults to all files.
- **offset** (optional): Line offset for checking/modifying files.
- **remove_offset** (optional): Number of characters to remove from the beginning of the file.
- **debug** (optional): Enables verbose logging for debugging.
- **use_memory_map** (optional): Uses memory-mapped files for performance optimization.
- **fix** (optional): Automatically applies fixes instead of just reporting issues.

### Integrate `cr_checker` using Bazel module

The CopyRight check tool is integrated in other bazel repository using Bazel modules mechanism. The current tool is not registered within BCR so private Bazel registry needs to be select. To select custom Bazel registry add following lines into .bazelrc:

```python
common --registry=https://raw.githubusercontent.com/eclipse-score/bazel_registry/main/
common --registry=https://bcr.bazel.build
```

This will allow Bazel to look into project Bazel registry. After that all what is needed is to add following lines in MODULE.bazel:

```python
###############################################################################
#
# CopyRight checker dependencies
#
###############################################################################
bazel_dep(name = "score_cr_checker", version = "0.2.2")
```
