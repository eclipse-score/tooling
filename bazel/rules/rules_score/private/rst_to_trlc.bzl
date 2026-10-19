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

"""Bazel rule converting RST requirement directives to TRLC files."""

load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

def _rst_to_trlc_impl(ctx):
    """Convert each .rst source file to a .trlc file via the Python converter."""
    outs = []
    for i, src in enumerate(ctx.files.srcs):
        # The index keeps equally named sources of different directories apart.
        out = ctx.actions.declare_file("{}/{}/{}.trlc".format(ctx.label.name, i, src.basename.removesuffix(".rst")))
        outs.append(out)

        args = ctx.actions.args()
        args.add(src.path)
        args.add("--output", out)
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
    doc = """Converts sphinx-needs RST requirement directives to TRLC source files.

    Supported directives: ``assumed_system_req``, ``feat_req``, ``comp_req`` and
    ``aou_req``. Emits one ``.trlc`` file per ``.rst`` source; list the target in the
    ``srcs`` of ``assumed_system_requirements``, ``feature_requirements``,
    ``component_requirements`` or ``assumptions_of_use``.
    """,
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
                doc = "TRLC package of the records that ``satisfies`` / ``derived_from`` refer to; every reference of every source is resolved in this package.",
            ),
            "package": attr.string(
                default = "",
                doc = "TRLC package of the generated files; defaults to the CamelCased stem of each source (``index.rst`` -> ``Index``).",
            ),
        },
        **VERBOSITY_ATTR
    ),
)
