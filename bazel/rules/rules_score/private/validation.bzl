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

"""Shared helpers for running validation profiles from SCORE Bazel rules."""

PROFILES = struct(
    ARCHITECTURAL_DESIGN = "architectural-design",
    DEPENDABLE_ELEMENT = "dependable-element",
    UNIT = "unit",
)

VALIDATION_ATTRS = {
    "_validation_cli": attr.label(
        default = Label("//validation/core:validation_cli"),
        executable = True,
        cfg = "exec",
        doc = "Validation CLI executable.",
    ),
}

def _validation_log_name(ctx, profile):
    profile_name = profile.replace("-", "_")
    package_name = ctx.label.package.replace("/", "_")
    if package_name:
        return "{}.{}.{}.log".format(profile_name, package_name, ctx.label.name)
    return "{}.{}.log".format(profile_name, ctx.label.name)

def run_validation(
        ctx,
        *,
        validation_cli,
        profile,
        input_bundle,
        inputs,
        mnemonic,
        maturity,
        log_level):
    """Run the validation CLI for a profile-owned input bundle.

    Args:
        ctx: Rule context.
        validation_cli: Validation CLI executable File.
        profile: Validation CLI profile name.
        input_bundle: Dictionary serialized to validation_inputs.json.
        inputs: Input File objects required by the validation action.
        mnemonic: Action mnemonic for the validation action.
        maturity: Validation maturity policy. Development emits warnings and continues.
        log_level: Validation CLI log level.

    Returns:
        Struct with file and name fields describing the validation log entry.
    """

    validation_inputs = ctx.actions.declare_file(ctx.label.name + "/validation_inputs.json")
    validation_log = ctx.actions.declare_file(ctx.label.name + "/validation.log")

    ctx.actions.write(
        output = validation_inputs,
        content = json.encode_indent(input_bundle, indent = "  "),
    )

    validation_args = ctx.actions.args()
    validation_args.add("--profile", profile)
    validation_args.add("--inputs", validation_inputs)
    validation_args.add("--output", validation_log)
    validation_args.add("--log-level", log_level)
    if maturity == "development":
        validation_args.add("--warn-on-errors")

    ctx.actions.run(
        inputs = [validation_inputs] + inputs,
        outputs = [validation_log],
        executable = validation_cli,
        arguments = [validation_args],
        progress_message = "Running {} validation: {}".format(profile, ctx.label.name),
        mnemonic = mnemonic,
    )

    return struct(
        file = validation_log,
        name = _validation_log_name(ctx, profile),
    )
