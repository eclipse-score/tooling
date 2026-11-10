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
"""Expose the resolved score_tooling module version to report generation."""

# lobster-trace: UseCases.Report_Tooling_Version
def resolved_tooling_version(modules):
    """Return the tooling version, not the consuming module's version."""
    for module in modules:
        if module.name == "score_tooling":
            return module.version or "unavailable (unversioned module)"
    fail("Tooling provenance requires the score_tooling module to use this extension.")

def _tooling_provenance_repo_impl(repository_ctx):
    repository_ctx.file("BUILD.bazel", "")
    repository_ctx.file(
        "provenance.bzl",
        "TOOLING_VERSION = {}\n".format(repr(repository_ctx.attr.version)),
    )

_tooling_provenance_repo = repository_rule(
    implementation = _tooling_provenance_repo_impl,
    attrs = {"version": attr.string(mandatory = True)},
)

def _tooling_provenance_impl(module_ctx):
    _tooling_provenance_repo(
        name = "tooling_provenance",
        version = resolved_tooling_version(module_ctx.modules),
    )
    return module_ctx.extension_metadata(reproducible = True)

tooling_provenance = module_extension(
    implementation = _tooling_provenance_impl,
    doc = "Provides the resolved tooling version without consumer configuration.",
)
