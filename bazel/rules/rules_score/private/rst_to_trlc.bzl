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

"""Bazel rule and helper macro for converting RST requirement directives to TRLC files."""

load("@trlc//:trlc.bzl", "trlc_requirements")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

def rst_srcs_to_trlc(name, srcs, deps = [], ref_package = ""):
    """Convert any .rst entries in srcs to trlc_requirements targets.

    For each .rst entry a pair of intermediate targets is generated:
      - A rst_to_trlc conversion target (produces the .trlc file)
      - A trlc_requirements target (provides TrlcProviderInfo with the score model spec)

    Non-.rst entries are passed through unchanged (assumed to already be
    trlc_requirements labels providing TrlcProviderInfo).

    Args:
        name: Base name of the enclosing macro target, used to derive
            unique names for the generated intermediaries.
        srcs: Mixed list of .rst file paths and/or trlc_requirements labels.
        deps: trlc_requirements labels to include as deps when wrapping
            generated .trlc files (e.g. parent requirement packages).
        ref_package: TRLC package prefix for derived_from cross-references
            written into the generated .trlc content.

    Returns:
        List of srcs where .rst entries are replaced by generated trlc labels.
    """
    result = []
    for i, src in enumerate(srcs):
        if src.endswith(".rst"):
            gen_name = "_{}_rst_gen_{}".format(name, i)
            trlc_name = "_{}_trlc_{}".format(name, i)
            rst_to_trlc(
                name = gen_name,
                srcs = [src],
                ref_package = ref_package,
            )
            trlc_requirements(
                name = trlc_name,
                srcs = [":" + gen_name],
                spec = [Label("//bazel/rules/rules_score/trlc/config:score_requirements_model")],
                deps = deps,
            )
            result.append(":" + trlc_name)
        else:
            result.append(src)
    return result

def _rst_to_trlc_impl(ctx):
    """Convert each .rst source file to a .trlc file via the Python converter."""
    outs = []
    for src in ctx.files.srcs:
        out = ctx.actions.declare_file(src.basename[:-4] + ".trlc", sibling = src)
        outs.append(out)

        args = ctx.actions.args()
        args.add(src.path)
        args.add("--output-dir")
        args.add(out.dirname)
        if ctx.attr.ref_package:
            args.add("--ref-package")
            args.add(ctx.attr.ref_package)
        if ctx.attr.package:
            args.add("--package")
            args.add(ctx.attr.package)
        args.add("--log-level")
        args.add(get_log_level(ctx))

        ctx.actions.run(
            executable = ctx.executable._converter,
            inputs = [src],
            outputs = [out],
            arguments = [args],
            mnemonic = "RstToTrlc",
            progress_message = "Converting %s to TRLC" % src.short_path,
        )

    return [DefaultInfo(files = depset(outs))]

rst_to_trlc = rule(
    implementation = _rst_to_trlc_impl,
    doc = "Converts RST requirement directives to TRLC source files.",
    attrs = dict(
        {
            "srcs": attr.label_list(
                allow_files = [".rst"],
                mandatory = True,
                doc = "RST files containing supported requirement directives.",
            ),
            "_converter": attr.label(
                default = Label("//bazel/rules/rules_score:rst_to_trlc"),
                executable = True,
                allow_files = True,
                cfg = "exec",
            ),
            "ref_package": attr.string(
                default = "",
                doc = "TRLC package prefix used for derived_from cross-references.",
            ),
            "package": attr.string(
                default = "",
                doc = "Optional TRLC package name override; defaults to the input file stem.",
            ),
        },
        **VERBOSITY_ATTR
    ),
)
