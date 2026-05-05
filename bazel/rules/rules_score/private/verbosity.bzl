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

"""Common verbosity helpers for rules_score rules.

Provides a shared attribute definition and accessor function so that every
rules_score build rule can expose the same ``--log-level`` argument to its
underlying tools via the ``//bazel/rules/rules_score:verbosity`` build
setting.

Usage in a rule definition::

    load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

    my_rule = rule(
        implementation = _impl,
        attrs = dict(
            ...,
            **VERBOSITY_ATTR
        ),
    )

    def _impl(ctx):
        log_level = get_log_level(ctx)  # returns "warn", "info", or "debug"
        ctx.actions.run(
            ...,
            arguments = ["--log-level", log_level, ...],
        )
"""

load("@bazel_skylib//rules:common_settings.bzl", "BuildSettingInfo")

# Private attribute that reads the verbosity build setting.
# Merge into a rule's attrs dict with ``**VERBOSITY_ATTR``.
VERBOSITY_ATTR = {
    "_verbosity": attr.label(
        default = Label("//bazel/rules/rules_score:verbosity"),
        doc = "Verbosity level build setting (warn/info/debug).",
    ),
}

def get_log_level(ctx):
    """Return the current log level string from the build setting.

    Args:
        ctx: Rule context (must have ``_verbosity`` in its attrs).

    Returns:
        One of ``"warn"``, ``"info"``, or ``"debug"``.
    """
    return ctx.attr._verbosity[BuildSettingInfo].value
