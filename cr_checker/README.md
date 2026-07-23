<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

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
- **-c**, **--config-file**: Path to a config file used to render template placeholders (e.g. author). Optional; an invalid file is a hard error.
- **--exclusion-file**: Path to a file listing paths to exclude from the copyright check.
- **-v**, **--verbose**: Enable debug-level logging.
- **-l**, **--log-file**: Path to a log file where logs will be saved. If not provided, logs will print to the console.
- **-e**, **--extensions**: List of file extensions to filter, e.g., -e py cpp.
- **--use_memory_map**: Use memory-mapped file reading for large files (check mode only; `--fix` always reads the whole file).
- **--encoding**: File encoding (default is utf-8).
- **--offset**: Force this many characters (plus any trailing blank lines) at the start of the file to be treated as a recognized preamble, overriding auto-detection. Character-based, not byte-based. Rarely needed: a leading shebang is detected and preserved automatically; use this only for other preamble kinds the tool doesn't (yet) recognize.
- **-f**, **--fix**: Setting script into fix mode where copyright header will be added to the files if it's missing from same.
- **--remove-offset**: Number of characters to remove before appending proper copyright header (works only with `--fix` option).
- **--force**: With `--fix`, also rewrite headers whose similarity to the template is below the auto-fix threshold (normally left untouched and only reported, since they may be a genuinely different license text). Never affects a duplicate-header file, which always requires manual review. Ignored without `--fix`.
- **--modified-only**: Only check files that differ from `HEAD` (staged and/or unstaged), e.g. for a fast, incremental pre-commit run. Takes precedence over `inputs`.
- **inputs**: Directories or files to parse, or a parameter file prefixed with @ that lists files or directories. Optional -- when omitted, the whole repository (per `git ls-files`) is checked.

> NOTE: Option `--remove-offset` can have severe consequences if the offset is miscalculated. Use with **extreme caution**.

> NOTE: Option `--force` bypasses the safety guard that normally prevents `--fix` from overwriting a header that looks substantially different from the template -- it may not be the same license at all. **Always review the diff afterwards.**

> NOTE: Setting directory as `.` will cause that tool removes your complete workspace! This is connected with how Bazel includes python into build. **DO NOT USE THIS OPTION UNLESS YOU'RE 100% SURE IN WHAT YOU'RE DOING**.

### Examples

```sh
python cr_checker.py -t templates.ini -e py cpp -v -l logs.txt my_random_file.cpp my_random_file.py

python cr_checker.py -t templates.ini -e py cpp --offset 24 --use_memory_map @files_to_check.txt

python cr_checker.py -t templates.ini -e py cpp --fix --offset 24 --use_memory_map @files_to_check.txt

```

#### A bit more about `--offset`

A leading shebang (`#!/usr/bin/env python3`, etc.) is detected and preserved
automatically -- you don't need `--offset` for it. For example, given:

```python
#!/usr/bin/env python3

import os
```

running:

```sh
python cr_checker.py -t templates.ini -e py cpp --fix @files_to_check.txt
```

produces:

```python
#!/usr/bin/env python3

##################
# COPYRIGHT HEADER
##################

import os
```

#### A bit more about `--force`

`--fix` only rewrites a wrong-format header automatically when it's similar
enough to the rendered template (a formatting drift -- border style, a
missing angle bracket, a stray typo, ...). A header that scores below the
threshold is left untouched and only reported, since it could be a genuinely
different, unrelated license text that must never be silently overwritten.

If you've confirmed (e.g. via `--verbose`, which logs the similarity
percentage for every non-compliant file) that the low-scoring headers are
actually just an old/outdated version of the same copyright statement --
e.g. a header predating a template change, or one written with a different
comment-wrapper convention -- pass `--force` to rewrite them anyway:

```sh
python cr_checker.py -t templates.ini --fix --force @files_to_check.txt
```

When invoking through the Bazel `.fix` target, remember to put `--` before
the flag so Bazel forwards it to the tool instead of trying to parse it
itself:

```sh
bazel run //:copyright.fix -- --force
```

`--force` never touches a file with a *duplicate* copyright header --
that always requires manual review, regardless of similarity. **Always
review the resulting diff afterwards**, since a low similarity score can
also mean the existing text is a genuinely different, unrelated license.

`--offset=<NUM>` is only needed to force-treat a preamble kind the tool
doesn't recognize (e.g. content that isn't a shebang) as content that must
stay above the header, or to tell the tool where to start looking for an
existing header when it isn't at the very top of the file. Content between
the (auto-detected or forced) offset and an existing header that isn't part
of either is reported as a *misplaced* header and, with `--fix`, moved to
immediately after the newly written header.

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
load("@score_tooling//cr_checker:cr_checker.bzl", "copyright_checker")
copyright_checker(
    name = "copyright_check",
    srcs = glob(["src/**/*.cpp", "src/**/*.h"]),
    config = "@score_tooling//cr_checker/resources:config",
    template = "@score_tooling//cr_checker/resources:templates",
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

`cr_checker` is distributed as part of the unified `score_tooling` Bazel module (it is no longer
a standalone `score_cr_checker` module). The current module is not registered within BCR so a
private Bazel registry needs to be selected. To select the custom Bazel registry, add the
following lines into `.bazelrc`:

```python
common --registry=https://raw.githubusercontent.com/eclipse-score/bazel_registry/main/
common --registry=https://bcr.bazel.build
```

This will allow Bazel to look into the project Bazel registry. After that all what is needed is to add following lines in MODULE.bazel:

```python
###############################################################################
#
# CopyRight checker dependencies
#
###############################################################################
bazel_dep(name = "score_tooling", version = "1.0.0")
```
