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

""" Bazel rule for generating dash formatted requirements file
"""

def _impl(ctx):
    """ The implementation function of the rule.
    """

    output = ctx.actions.declare_file("formatted.txt")
    args = ctx.actions.args()
    args.add("-i", ctx.file.requirement_file)
    args.add("-o", output)
    args.add("-t", ctx.attr.file_type)
    if ctx.attr.skip_source_filter:
        args.add("--skip-source-filter")
    if ctx.attr.filter_keywords:
        args.add("--filter-keywords")
        args.add_all(ctx.attr.filter_keywords)

    ctx.actions.run(
        inputs = [ctx.file.requirement_file],
        outputs = [output],
        arguments = [args],
        progress_message = "Generating Dash formatted dependency file ...",
        mnemonic = "DashFormat",
        executable = ctx.executable._tool,
    )
    return DefaultInfo(files = depset([output]))

dash_format_converter = rule(
    implementation = _impl,
    attrs = {
        "requirement_file": attr.label(
            mandatory = True,
            allow_single_file = True,
            doc = "The requirement (requirement_lock.txt) input file which holds deps",
        ),
        "file_type": attr.string(
            default = "requirements",
            doc = "Type of input file: 'requirements' for requirements.txt or 'cargo' for Cargo.lock",
        ),
        "skip_source_filter": attr.bool(
            default = False,
            doc = "If True, skip filtering out packages based on source (no source check, no keyword filtering)",
        ),
        "filter_keywords": attr.string_list(
            default = ["dummy-source-keyword"],
            doc = "List of keywords to filter out from package sources. Packages containing any of these keywords will be excluded.",
        ),
        "_tool": attr.label(
            default = Label("@score_tooling//dash/tool/formatters:dash_format_converter"),
            executable = True,
            cfg = "exec",
            doc = "",
        ),
    },
)
