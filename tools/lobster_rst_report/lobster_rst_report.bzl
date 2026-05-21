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

"""Rule for generating multi-page RST traceability report from lobster inputs."""

load("@lobster//:lobster.bzl", "LobsterProvider")

def _lobster_report_rst_impl(ctx):
    # Expand lobster config template with actual sandbox paths from LobsterProvider
    lobster_config_substitutions = {}
    for input_target in ctx.attr.inputs:
        lobster_config_substitutions.update(input_target[LobsterProvider].lobster_input)

    lobster_config = ctx.actions.declare_file("{}_expanded_lobster.conf".format(ctx.attr.name))
    ctx.actions.expand_template(
        template = ctx.file.config,
        output = lobster_config,
        substitutions = lobster_config_substitutions,
    )

    # Run lobster-report to generate JSON
    lobster_report_json = ctx.actions.declare_file("{}_report.json".format(ctx.attr.name))
    report_args = ctx.actions.args()
    report_args.add_all(["--lobster-config", lobster_config.path])
    report_args.add_all(["--out", lobster_report_json.path])

    ctx.actions.run(
        executable = ctx.executable._lobster_report,
        inputs = depset(ctx.files.inputs + [lobster_config]),
        outputs = [lobster_report_json],
        arguments = [report_args],
        progress_message = "lobster-report %s" % lobster_report_json.path,
    )

    # Run lobster-rst-report to generate multi-page RST directory
    rst_dir = ctx.actions.declare_directory(ctx.attr.name)

    package = ctx.label.package
    package_depth = len(package.split("/")) if package else 0
    source_root = "/".join([".."] * (package_depth + 2)) + "/"

    rst_args = ctx.actions.args()
    rst_args.add(lobster_report_json.path)
    rst_args.add_all(["--out-dir", rst_dir.path])
    rst_args.add_all(["--source-root", source_root])

    ctx.actions.run(
        executable = ctx.executable._lobster_rst_report,
        inputs = [lobster_report_json],
        outputs = [rst_dir],
        arguments = [rst_args],
        progress_message = "lobster-rst-report (pages) %s" % ctx.label.name,
    )

    return [DefaultInfo(files = depset([rst_dir]))]

lobster_report_rst = rule(
    implementation = _lobster_report_rst_impl,
    attrs = {
        "config": attr.label(
            mandatory = True,
            allow_single_file = True,
            doc = "Lobster configuration file (.conf)",
        ),
        "inputs": attr.label_list(
            providers = [LobsterProvider],
            mandatory = True,
            doc = "Lobster input targets providing .lobster files",
        ),
        "_lobster_report": attr.label(
            default = "@lobster//:lobster-report",
            executable = True,
            cfg = "exec",
        ),
        "_lobster_rst_report": attr.label(
            default = "//tools/lobster_rst_report:lobster-rst-report",
            executable = True,
            cfg = "exec",
        ),
    },
)
