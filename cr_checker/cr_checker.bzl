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

"""Defines Bazel rules for running copyright checks and fixes.
"""

load("@aspect_rules_py//py:defs.bzl", "py_binary")

def copyright_checker(
        name,
        visibility,
        template,
        config,
        exclusion = None,
        srcs = [],
        extensions = [],
        offset = 0,
        remove_offset = 0,
        debug = False,
        use_memory_map = False,
        target_compatible_with = None):
    """
    Defines ``bazel run`` targets for checking and fixing copyright headers.

    Args:
        name (str): The name of the rule, used as an identifier in the build system.
        visibility (list): A list defining the visibility of the rule, specifying which
                           targets can use this rule.
        template (str): Path to the template resource used for validation.
        config (str): Path to the config resource used for project variables.
        exclusion (str, optional): Path to a text file listing files to be excluded from the copyright check.
                                   File format: one path per line, relative to the repository root.
        srcs (list, optional): Workspace-relative paths (files, directories, or git
                               pathspecs) to restrict which files are checked.
                               Defaults to an empty list, meaning the whole
                               repository (per ``git ls-files``) is checked.
        extensions (list, optional): A list of file extensions to filter the source files.
                                     Defaults to an empty list, meaning all files are checked.
        offset (int, optional): The line offset for applying checks or modifications.
                                Defaults to 0.
        remove_offset (int, optional): The line offset for removing chars from beginning of file.
                                Defaults to 0.
        debug (bool, optional): Whether to enable debug mode, providing additional logs.
                                Defaults to False.
        use_memory_map (bool, optional): Whether to use memory mapping for large files to
                                         improve performance. Defaults to False.
        target_compatible_with (list, optional): Standard Bazel target compatibility constraint.

    Returns:
        None: This function defines a rule for a build system and does not return a value.
    """
    common_args = [
        "-t $(location {})".format(template),
        "-c $(location {})".format(config),
    ]
    if extensions:
        common_args.append("-e {}".format(" ".join(extensions)))
    if exclusion:
        common_args.append("--exclusion-file $(location {})".format(exclusion))
    if offset:
        common_args.append("--offset {}".format(offset))
    if debug:
        common_args.append("-v")
    if use_memory_map:
        common_args.append("--use_memory_map")

    # Optional scope restriction, forwarded as-is to `git ls-files` pathspecs.
    common_args.extend(srcs)

    # Only the tool resources need to be in runfiles; the files to check are
    # discovered directly from the workspace at runtime via `git ls-files`.
    data = [template, config]
    if exclusion:
        data.append(exclusion)

    py_binary(
        name = "{}.check".format(name),
        main = "cr_checker.py",
        srcs = [
            "@score_tooling//cr_checker/tool:cr_checker_lib",
        ],
        deps = [
            "@score_tooling//cr_checker/tool:cr_checker_lib",
        ],
        args = common_args,
        data = data,
        visibility = visibility,
        target_compatible_with = target_compatible_with,
    )

    fix_args = ["--fix"] + common_args
    if remove_offset:
        fix_args.append("--remove-offset {}".format(remove_offset))

    py_binary(
        name = "{}.fix".format(name),
        main = "cr_checker.py",
        srcs = [
            "@score_tooling//cr_checker/tool:cr_checker_lib",
        ],
        deps = [
            "@score_tooling//cr_checker/tool:cr_checker_lib",
        ],
        args = fix_args,
        data = data,
        visibility = visibility,
        target_compatible_with = target_compatible_with,
    )

    native.alias(
        name = "copyright-check",
        actual = ":" + name + ".check",
        visibility = visibility,
        target_compatible_with = target_compatible_with,
        tags = [
            "cli_help=Check for license headers:\n" +
            "bazel run //:copyright-check",
        ],
    )

    native.alias(
        name = "copyright-fix",
        actual = ":" + name + ".fix",
        visibility = visibility,
        tags = [
            "cli_help=Fix license headers:\n" +
            "bazel run //:copyright-fix",
        ],
    )
